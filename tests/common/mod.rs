#![allow(dead_code)]

use laposta_rs::Client;
use wiremock::{MockServer, ResponseTemplate};

pub const AUTH: &str = "Basic dGVzdC1rZXk6";

pub async fn setup() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let client = Client::builder()
        .api_key("test-key")
        .base_url(format!("{}/v2/", server.uri()))
        .build()
        .unwrap();
    (server, client)
}

pub fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

pub fn respond(name: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(fixture(name), "application/json")
}

pub fn error(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}
