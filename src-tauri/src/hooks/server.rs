//! Local HTTP endpoint for the hooks injected through `--settings`. Binds to 127.0.0.1 on a random
//! port and accepts only requests carrying a session key and its token.

use std::io::Read;
use std::sync::Arc;
use std::thread;

use tiny_http::{Header, Method, Request, Response, Server};

use super::payload::HookPayload;
use super::tokens::TokenRegistry;

pub const SESSION_HEADER: &str = "X-CCM-Session";
pub const TOKEN_HEADER: &str = "X-CCM-Token";
pub const HOOK_PATH: &str = "/hook";
const MAX_BODY_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HookEvent {
    pub session_key: String,
    pub payload: HookPayload,
}

pub type HookHandler = Arc<dyn Fn(HookEvent) + Send + Sync>;

pub struct HookServer {
    port: u16,
}

impl HookServer {
    pub fn start(tokens: Arc<TokenRegistry>, handler: HookHandler) -> std::io::Result<Self> {
        let server = Server::http("127.0.0.1:0").map_err(std::io::Error::other)?;
        let port = server.server_addr().to_ip().map(|address| address.port()).ok_or_else(|| std::io::Error::other("endereço sem porta"))?;
        thread::Builder::new().name("hook-server".to_owned()).spawn(move || {
            for request in server.incoming_requests() {
                handle(request, &tokens, &handler);
            }
        })?;
        Ok(Self { port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

fn header_value(request: &Request, name: &'static str) -> Option<String> {
    request.headers().iter().find(|header| header.field.equiv(name)).map(|header| header.value.as_str().to_owned())
}

fn reply(request: Request, status: u16, body: &str) {
    let json = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).ok();
    let mut response = Response::from_string(body).with_status_code(status);
    if let Some(json) = json {
        response.add_header(json);
    }
    if let Err(error) = request.respond(response) {
        log::debug!("falha ao responder hook: {error}");
    }
}

/// Always answers `{}` on success: the app observes hooks and never takes decisions for Claude Code.
fn handle(mut request: Request, tokens: &TokenRegistry, handler: &HookHandler) {
    if request.url() != HOOK_PATH {
        return reply(request, 404, "{}");
    }
    if *request.method() != Method::Post {
        return reply(request, 405, "{}");
    }
    let session_key = header_value(&request, SESSION_HEADER).unwrap_or_default();
    let token = header_value(&request, TOKEN_HEADER).unwrap_or_default();
    if !tokens.verify(&session_key, &token) {
        return reply(request, 401, "{}");
    }
    if request.body_length().is_some_and(|length| length as u64 > MAX_BODY_BYTES) {
        return reply(request, 413, "{}");
    }
    let mut body = Vec::new();
    let read = request.as_reader().take(MAX_BODY_BYTES + 1).read_to_end(&mut body);
    if read.is_err() || body.len() as u64 > MAX_BODY_BYTES {
        return reply(request, 413, "{}");
    }
    let Ok(payload) = serde_json::from_slice::<HookPayload>(&body) else {
        return reply(request, 400, "{}");
    };
    reply(request, 200, "{}");
    handler(HookEvent { session_key, payload });
}
