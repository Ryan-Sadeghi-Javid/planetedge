use std::{convert::Infallible, net::SocketAddr, time::Instant};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use hyper::body::to_bytes;
use hyper::client::HttpConnector;
use hyper::service::service_fn;
use hyper::{Body, Client, Server};
use lazy_static::lazy_static;
use prometheus::{Encoder, IntCounterVec, IntGauge, Opts, Registry, TextEncoder};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

use planetedge_sdk::{EdgeContext, EdgePlugin, Nop};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref ACTIVE_CONN: IntGauge = IntGauge::new("active_connections", "Active TCP connections").unwrap();
    static ref REQ_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("requests_total", "Total requests handled"),
        &["route", "status"]
    ).unwrap();
}

fn prometheus_handler() -> Result<Response<Body>, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    fmt().with_env_filter(filter).init();

    // metrics
    REGISTRY.register(Box::new(ACTIVE_CONN.clone())).ok();
    REGISTRY.register(Box::new(REQ_TOTAL.clone())).ok();

    let origin = std::env::var("ORIGIN_URL").unwrap_or_else(|_| "https://httpbin.org".to_string());
    info!(%origin, "starting PlanetEdge proxy");

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;

    // shared client to the origin
    let client: Client<HttpConnector, Body> = Client::new();

    // simple plugin chain with a single NOP
    let plugins: Vec<Box<dyn EdgePlugin>> = vec![Box::new(Nop)];

    let make_svc = hyper::service::make_service_fn(move |_conn| {
        let client = client.clone();
        let origin = origin.clone();
        let plugins = plugins.iter().map(|p| p.name()).collect::<Vec<_>>().join(",");
        async move {
            Ok::<_, Infallible>(service_fn(move |mut req: Request<Body>| {
                let client = client.clone();
                let origin = origin.clone();
                let plugins = plugins.clone();

                async move {
                    if req.uri().path() == "/metrics" {
                        return prometheus_handler();
                    }

                    ACTIVE_CONN.inc();
                    let started = Instant::now();

                    // Build context
                    let route_id = "default";
                    let ctx = EdgeContext {
                        route_id: route_id.to_string(),
                        request_id: Uuid::new_v4().to_string(),
                    };

                    // Buffer body to apply plugins (simple first-pass approach)
                    let body_bytes: Bytes = to_bytes(req.body_mut()).await.unwrap_or_default();
                    let req2 = Request::builder()
                        .method(req.method())
                        .uri(format!("{origin}{}", req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/")))
                        .version(req.version())
                        .body(body_bytes.to_vec())
                        .unwrap();

                    // on_request plugins (NOP now)
                    let nop = planetedge_sdk::Nop;
                    let req2 = nop.on_request(&ctx, req2);

                    // Forward to origin
                    let mut out_req = Request::builder()
                        .method(req2.method().clone())
                        .uri(req2.uri().to_string())
                        .body(Body::from(req2.body().clone()))
                        .unwrap();

                    // copy headers
                    *out_req.headers_mut() = req.headers().clone();

                    let res = match client.request(out_req).await {
                        Ok(mut r) => {
                            let status = r.status();
                            let res_body = to_bytes(r.body_mut()).await.unwrap_or_default();
                            let new_res = Response::builder()
                                .status(status)
                                .body(res_body.to_vec())
                                .unwrap();

                            // on_response plugins
                            let new_res = nop.on_response(&ctx, new_res);

                            // build hyper response
                            let mut final_res = Response::builder()
                                .status(new_res.status())
                                .body(Body::from(new_res.body().clone()))
                                .unwrap();
                            *final_res.headers_mut() = r.headers().clone();

                            final_res
                        }
                        Err(e) => {
                            error!("upstream error: {e}");
                            Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(Body::from("upstream error"))
                                .unwrap()
                        }
                    };

                    let code = res.status().as_u16().to_string();
                    REQ_TOTAL.with_label_values(&[route_id, &code]).inc();

                    let elapsed_ms = started.elapsed().as_millis();
                    info!(%plugins, req_id=%ctx.request_id, method=%req.method(), path=%req.uri().path(), status=%res.status().as_u16(), elapsed_ms, "handled request");

                    ACTIVE_CONN.dec();
                    Ok::<_, Infallible>(res)
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    info!("listening on http://{addr}");
    if let Err(e) = server.await {
        error!("server error: {e}");
    }
    Ok(())
}
