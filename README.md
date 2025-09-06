# PlanetEdge

A QUIC-ready, programmable edge reverse proxy (Rust) with a control plane and plugin runtime.

## Quick demo (HTTP/1.1 path)
- Proxies incoming requests to an origin (set `ORIGIN_URL`).
- Basic routing and middleware chain with a pluggable interface.
- Prometheus `/metrics` endpoint + structured logs.

## Next milestones
- [ ] Enable QUIC/HTTP3 data path (feature flag: `http3`) using `quinn` + `h3`.
- [ ] WASM plugin runtime (wasmtime) + example plugins (JWT auth, header rewrite).
- [ ] Raft (openraft) control plane for global config/versioning.
- [ ] Canary deploys + instant rollback, SLO guardrails, eBPF probes.
- [ ] Multi-region K8s + Terraform with chaos demo.

## Structure
```
planetedge/
├─ Cargo.toml                   # workspace
├─ README.md
├─ planetedge-proxy/            # data plane (runs today over HTTP/1.1)
├─ planetedge-control/          # control plane API (skeleton)
├─ planetedge-sdk/              # plugin SDK (traits, types)
├─ plugins/
│  └─ example-header-rewrite/   # placeholder for a WASM plugin
├─ infra/
│  ├─ k8s/                      # manifests (placeholders)
│  └─ terraform/                # IaC (placeholders)
└─ planetedge-ui/               # admin dashboard (placeholder)
```

## Build & run (proxy)
```bash
cd planetedge-proxy
export ORIGIN_URL="https://httpbin.org"
cargo run
# in another shell
curl -i http://127.0.0.1:8080/get
curl -s http://127.0.0.1:8080/metrics | head
```
