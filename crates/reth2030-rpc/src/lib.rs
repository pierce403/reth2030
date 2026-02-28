//! JSON-RPC and Engine API scaffolding for `reth2030`.

use axum::{
    extract::State,
    http::{header, HeaderMap},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct RpcServerState {
    pub client_version: String,
    pub chain_id: u64,
    pub block_number: u64,
    pub engine_jwt: String,
}

impl Default for RpcServerState {
    fn default() -> Self {
        Self {
            client_version: "reth2030/v0.1.0".to_string(),
            chain_id: 1,
            block_number: 0,
            engine_jwt: "dev-jwt".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

pub fn router(state: RpcServerState) -> Router {
    Router::new()
        .route("/", post(handle_rpc))
        .route("/engine", post(handle_engine_rpc))
        .with_state(state)
}

pub async fn serve(addr: SocketAddr, state: RpcServerState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state)).await
}

async fn handle_rpc(State(state): State<RpcServerState>, body: String) -> Json<JsonRpcResponse> {
    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(response) => return Json(response),
    };

    Json(dispatch_public_method(&state, request))
}

async fn handle_engine_rpc(
    State(state): State<RpcServerState>,
    headers: HeaderMap,
    body: String,
) -> Json<JsonRpcResponse> {
    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(response) => return Json(response),
    };

    if !is_engine_authorized(&headers, &state.engine_jwt) {
        return Json(error_response(
            request.id,
            -32001,
            "Engine API authorization failed",
        ));
    }

    Json(dispatch_engine_method(request))
}

fn decode_request(body: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| error_response(Value::Null, -32700, "Parse error"))?;

    let request: JsonRpcRequest = serde_json::from_value(value)
        .map_err(|_| error_response(Value::Null, -32600, "Invalid request"))?;

    if request.jsonrpc != "2.0" {
        return Err(error_response(
            request.id,
            -32600,
            "Invalid request: jsonrpc must be 2.0",
        ));
    }

    Ok(request)
}

fn dispatch_public_method(state: &RpcServerState, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "web3_clientVersion" => ok_response(request.id, json!(state.client_version)),
        "eth_chainId" => ok_response(request.id, json!(format!("0x{:x}", state.chain_id))),
        "eth_blockNumber" => ok_response(request.id, json!(format!("0x{:x}", state.block_number))),
        _ => error_response(request.id, -32601, "Method not found"),
    }
}

fn dispatch_engine_method(request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "engine_exchangeCapabilities" => ok_response(
            request.id,
            json!([
                "engine_newPayloadV3",
                "engine_forkchoiceUpdatedV3",
                "engine_getPayloadV3"
            ]),
        ),
        "engine_forkchoiceUpdatedV3" => ok_response(
            request.id,
            json!({
                "payloadStatus": {
                    "status": "SYNCING"
                }
            }),
        ),
        _ => error_response(request.id, -32601, "Method not found"),
    }
}

fn is_engine_authorized(headers: &HeaderMap, expected_jwt: &str) -> bool {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    auth == format!("Bearer {}", expected_jwt)
}

fn ok_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Value, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{router, JsonRpcResponse, RpcServerState};
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    fn rpc_request(method: &str) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": []
        })
        .to_string()
    }

    async fn call_json_rpc(uri: &str, body: String, auth_header: Option<&str>) -> JsonRpcResponse {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");

        if let Some(auth) = auth_header {
            builder = builder.header(header::AUTHORIZATION, auth);
        }

        let request = builder.body(Body::from(body)).expect("request build");

        let response = router(RpcServerState::default())
            .oneshot(request)
            .await
            .expect("router response");

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("body collect")
            .to_bytes();

        serde_json::from_slice::<JsonRpcResponse>(&body_bytes).expect("json-rpc response decode")
    }

    #[tokio::test]
    async fn serves_baseline_public_methods() {
        let client_version = call_json_rpc("/", rpc_request("web3_clientVersion"), None).await;
        assert_eq!(client_version.jsonrpc, "2.0");
        assert_eq!(client_version.id, Value::from(1));
        assert_eq!(
            client_version.result.expect("result"),
            Value::from("reth2030/v0.1.0")
        );
        assert!(client_version.error.is_none());

        let chain_id = call_json_rpc("/", rpc_request("eth_chainId"), None).await;
        assert_eq!(chain_id.result.expect("result"), Value::from("0x1"));

        let block_number = call_json_rpc("/", rpc_request("eth_blockNumber"), None).await;
        assert_eq!(block_number.result.expect("result"), Value::from("0x0"));
    }

    #[tokio::test]
    async fn unknown_public_method_returns_structured_error() {
        let response = call_json_rpc("/", rpc_request("eth_unknownMethod"), None).await;

        let error = response.error.expect("error object");
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert!(response.result.is_none());
    }

    #[tokio::test]
    async fn engine_api_requires_bearer_token() {
        let unauthorized =
            call_json_rpc("/engine", rpc_request("engine_exchangeCapabilities"), None).await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(unauthorized_error.code, -32001);
        assert_eq!(
            unauthorized_error.message,
            "Engine API authorization failed"
        );

        let authorized = call_json_rpc(
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            Some("Bearer dev-jwt"),
        )
        .await;
        assert!(authorized.error.is_none());
        assert_eq!(
            authorized.result.expect("result"),
            json!([
                "engine_newPayloadV3",
                "engine_forkchoiceUpdatedV3",
                "engine_getPayloadV3"
            ])
        );
    }
}
