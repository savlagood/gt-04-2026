use std::sync::{Arc, RwLock};

use tiny_http::{Header, Response, Server};

use crate::api::dto::{LogMessage, PlayerResponse};

pub type SharedArena = Arc<RwLock<Option<PlayerResponse>>>;
pub type SharedLogs = Arc<RwLock<Vec<LogMessage>>>;

pub fn start(port: u16, arena: SharedArena, logs: SharedLogs) {
    let server = Server::http(format!("0.0.0.0:{port}")).expect("proxy server bind");
    tracing::info!("proxy listening on :{port}");
    let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();
    let cors_origin = Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap();
    let cors_methods = Header::from_bytes("Access-Control-Allow-Methods", "GET, OPTIONS").unwrap();
    let cors_headers = Header::from_bytes("Access-Control-Allow-Headers", "*").unwrap();
    for req in server.incoming_requests() {
        let method = req.method().as_str().to_owned();
        let url = req.url().to_owned();
        if method == "OPTIONS" {
            let _ = req.respond(
                Response::from_string("")
                    .with_status_code(204u16)
                    .with_header(cors_origin.clone())
                    .with_header(cors_methods.clone())
                    .with_header(cors_headers.clone()),
            );
            continue;
        }
        let (status, body) = match url.as_str() {
            "/api/arena" => {
                let guard = arena.read().unwrap();
                match &*guard {
                    Some(r) => (200u16, serde_json::to_string(r).unwrap()),
                    None => (503, r#"{"error":"not ready"}"#.to_owned()),
                }
            }
            "/api/logs" => {
                let guard = logs.read().unwrap();
                (200, serde_json::to_string(&*guard).unwrap())
            }
            _ => (404, r#"{"error":"not found"}"#.to_owned()),
        };
        let _ = req.respond(
            Response::from_string(body)
                .with_status_code(status)
                .with_header(content_type.clone())
                .with_header(cors_origin.clone()),
        );
    }
}
