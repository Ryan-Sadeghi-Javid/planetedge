//! SDK for PlanetEdge plugins (WASM-friendly API to be expanded).

use http::{Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeContext {
    pub route_id: String,
    pub request_id: String,
}

pub trait EdgePlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_request(&self, _ctx: &EdgeContext, req: Request<Vec<u8>>) -> Request<Vec<u8>> {
        req
    }
    fn on_response(&self, _ctx: &EdgeContext, res: Response<Vec<u8>>) -> Response<Vec<u8>> {
        res
    }
}

/// No-op plugin for demo
pub struct Nop;

impl EdgePlugin for Nop {
    fn name(&self) -> &'static str { "nop" }
}
