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

const ENGINE_AUTH_ERROR_CODE: i64 = -32001;
const ENGINE_NOT_IMPLEMENTED_CODE: i64 = -32004;
const ENGINE_CAPABILITIES: [&str; 3] = [
    "engine_newPayloadV3",
    "engine_forkchoiceUpdatedV3",
    "engine_getPayloadV3",
];

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
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
        Err(response) => return Json(*response),
    };

    Json(dispatch_public_method(&state, request))
}

async fn handle_engine_rpc(
    State(state): State<RpcServerState>,
    headers: HeaderMap,
    body: String,
) -> Json<JsonRpcResponse> {
    if let Err(auth_error) = authorize_engine_request(&headers, &state.engine_jwt) {
        return Json(engine_auth_error_response(Value::Null, auth_error));
    }

    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(response) => return Json(*response),
    };

    if !is_engine_method(&request.method) {
        return Json(error_response(request.id, -32601, "Method not found"));
    }

    Json(dispatch_engine_method(request))
}

fn decode_request(body: &str) -> Result<JsonRpcRequest, Box<JsonRpcResponse>> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| Box::new(error_response(Value::Null, -32700, "Parse error")))?;

    let request: JsonRpcRequest = serde_json::from_value(value)
        .map_err(|_| Box::new(error_response(Value::Null, -32600, "Invalid request")))?;

    if request.jsonrpc != "2.0" {
        return Err(Box::new(error_response(
            request.id,
            -32600,
            "Invalid request: jsonrpc must be 2.0",
        )));
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
        "engine_exchangeCapabilities" => ok_response(request.id, json!(ENGINE_CAPABILITIES)),
        "engine_newPayloadV3" => ok_response(
            request.id,
            json!({
                "status": "SYNCING",
                "latestValidHash": null,
                "validationError": "Placeholder execution path in scaffold"
            }),
        ),
        "engine_forkchoiceUpdatedV3" => ok_response(
            request.id,
            json!({
                "payloadStatus": {
                    "status": "SYNCING",
                    "latestValidHash": null,
                    "validationError": "Placeholder execution path in scaffold"
                },
                "payloadId": null
            }),
        ),
        "engine_getPayloadV3" => error_response(
            request.id,
            ENGINE_NOT_IMPLEMENTED_CODE,
            "Engine method placeholder not implemented",
        ),
        _ => error_response(request.id, -32601, "Method not found"),
    }
}

fn is_engine_method(method: &str) -> bool {
    method.starts_with("engine_")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineAuthError {
    MissingAuthorizationHeader,
    MultipleAuthorizationHeaders,
    InvalidAuthorizationHeaderEncoding,
    InvalidAuthorizationScheme,
    MissingBearerToken,
    InvalidAuthorizationFormat,
    InvalidToken,
    MissingConfiguredJwt,
}

impl EngineAuthError {
    const fn reason(self) -> &'static str {
        match self {
            Self::MissingAuthorizationHeader => "missing_authorization_header",
            Self::MultipleAuthorizationHeaders => "multiple_authorization_headers",
            Self::InvalidAuthorizationHeaderEncoding => "invalid_authorization_header_encoding",
            Self::InvalidAuthorizationScheme => "invalid_authorization_scheme",
            Self::MissingBearerToken => "missing_bearer_token",
            Self::InvalidAuthorizationFormat => "invalid_authorization_format",
            Self::InvalidToken => "invalid_token",
            Self::MissingConfiguredJwt => "missing_configured_jwt",
        }
    }
}

fn authorize_engine_request(
    headers: &HeaderMap,
    expected_jwt: &str,
) -> Result<(), EngineAuthError> {
    if expected_jwt.trim().is_empty() {
        return Err(EngineAuthError::MissingConfiguredJwt);
    }

    let mut auth_values = headers.get_all(header::AUTHORIZATION).iter();
    let first = auth_values
        .next()
        .ok_or(EngineAuthError::MissingAuthorizationHeader)?;
    if auth_values.next().is_some() {
        return Err(EngineAuthError::MultipleAuthorizationHeaders);
    }

    let auth = first
        .to_str()
        .map_err(|_| EngineAuthError::InvalidAuthorizationHeaderEncoding)?;
    let mut parts = auth.split_whitespace();
    let scheme = parts
        .next()
        .ok_or(EngineAuthError::InvalidAuthorizationFormat)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return Err(EngineAuthError::InvalidAuthorizationScheme);
    }

    let token = parts.next().ok_or(EngineAuthError::MissingBearerToken)?;
    if parts.next().is_some() {
        return Err(EngineAuthError::InvalidAuthorizationFormat);
    }

    if token != expected_jwt {
        return Err(EngineAuthError::InvalidToken);
    }

    Ok(())
}

fn engine_auth_error_response(id: Value, reason: EngineAuthError) -> JsonRpcResponse {
    error_response_with_data(
        id,
        ENGINE_AUTH_ERROR_CODE,
        "Engine API authorization failed",
        json!({ "reason": reason.reason() }),
    )
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
    error_response_with_data(id, code, message, Value::Null)
}

fn error_response_with_data(id: Value, code: i64, message: &str, data: Value) -> JsonRpcResponse {
    let data = if data.is_null() { None } else { Some(data) };
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        router, JsonRpcResponse, RpcServerState, ENGINE_AUTH_ERROR_CODE,
        ENGINE_NOT_IMPLEMENTED_CODE,
    };
    use axum::{
        body::Body,
        http::{header, HeaderValue, Request, StatusCode},
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

    async fn call_json_rpc(
        state: RpcServerState,
        uri: &str,
        body: String,
        auth_headers: &[&str],
    ) -> JsonRpcResponse {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");

        for auth in auth_headers {
            builder = builder.header(header::AUTHORIZATION, *auth);
        }

        let request = builder.body(Body::from(body)).expect("request build");
        call_with_request(state, request).await
    }

    async fn call_with_request(state: RpcServerState, request: Request<Body>) -> JsonRpcResponse {
        let response = router(state)
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
        let client_version = call_json_rpc(
            RpcServerState::default(),
            "/",
            rpc_request("web3_clientVersion"),
            &[],
        )
        .await;
        assert_eq!(client_version.jsonrpc, "2.0");
        assert_eq!(client_version.id, Value::from(1));
        assert_eq!(
            client_version.result.expect("result"),
            Value::from("reth2030/v0.1.0")
        );
        assert!(client_version.error.is_none());

        let chain_id = call_json_rpc(
            RpcServerState::default(),
            "/",
            rpc_request("eth_chainId"),
            &[],
        )
        .await;
        assert_eq!(chain_id.result.expect("result"), Value::from("0x1"));

        let block_number = call_json_rpc(
            RpcServerState::default(),
            "/",
            rpc_request("eth_blockNumber"),
            &[],
        )
        .await;
        assert_eq!(block_number.result.expect("result"), Value::from("0x0"));
    }

    #[tokio::test]
    async fn unknown_public_method_returns_structured_error() {
        let response = call_json_rpc(
            RpcServerState::default(),
            "/",
            rpc_request("eth_unknownMethod"),
            &[],
        )
        .await;

        let error = response.error.expect("error object");
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_none());
        assert!(response.result.is_none());
    }

    #[tokio::test]
    async fn engine_api_requires_authorization_header() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &[],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(unauthorized_error.code, ENGINE_AUTH_ERROR_CODE);
        assert_eq!(
            unauthorized_error.message,
            "Engine API authorization failed"
        );
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "missing_authorization_header" })
        );
        assert_eq!(unauthorized.id, Value::Null);
    }

    #[tokio::test]
    async fn engine_api_accepts_case_insensitive_bearer_scheme() {
        let authorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["bearer dev-jwt"],
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

    #[tokio::test]
    async fn engine_api_rejects_wrong_token() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Bearer wrong-token"],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(unauthorized_error.code, ENGINE_AUTH_ERROR_CODE);
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "invalid_token" })
        );
    }

    #[tokio::test]
    async fn engine_api_rejects_non_bearer_scheme() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Basic dev-jwt"],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(unauthorized_error.code, ENGINE_AUTH_ERROR_CODE);
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "invalid_authorization_scheme" })
        );
    }

    #[tokio::test]
    async fn engine_api_rejects_missing_bearer_token() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Bearer"],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "missing_bearer_token" })
        );
    }

    #[tokio::test]
    async fn engine_api_rejects_malformed_authorization_format() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Bearer dev-jwt trailing"],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "invalid_authorization_format" })
        );
    }

    #[tokio::test]
    async fn engine_api_rejects_duplicate_authorization_headers() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Bearer dev-jwt", "Bearer dev-jwt"],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "multiple_authorization_headers" })
        );
    }

    #[tokio::test]
    async fn engine_api_rejects_non_utf8_authorization_header() {
        let request = Request::builder()
            .method("POST")
            .uri("/engine")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(rpc_request("engine_exchangeCapabilities")))
            .expect("request build");
        let (mut parts, body) = request.into_parts();
        parts.headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_bytes(&[0x42, 0x65, 0x61, 0x72, 0x65, 0x72, 0x20, 0xff])
                .expect("header value"),
        );

        let unauthorized =
            call_with_request(RpcServerState::default(), Request::from_parts(parts, body)).await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "invalid_authorization_header_encoding" })
        );
    }

    #[tokio::test]
    async fn unauthorized_engine_requests_do_not_parse_request_body() {
        let unauthorized = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            "not-json".to_string(),
            &[],
        )
        .await;
        let unauthorized_error = unauthorized.error.expect("error object");
        assert_eq!(unauthorized_error.code, ENGINE_AUTH_ERROR_CODE);
        assert_eq!(
            unauthorized_error.data.expect("reason"),
            json!({ "reason": "missing_authorization_header" })
        );
    }

    #[tokio::test]
    async fn authorized_engine_requests_parse_json_after_auth() {
        let parse_error = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            "not-json".to_string(),
            &["Bearer dev-jwt"],
        )
        .await;
        let parse_error_body = parse_error.error.expect("error");
        assert_eq!(parse_error_body.code, -32700);
        assert_eq!(parse_error_body.message, "Parse error");
    }

    #[tokio::test]
    async fn engine_namespace_rejects_non_engine_methods() {
        let response = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("web3_clientVersion"),
            &["Bearer dev-jwt"],
        )
        .await;
        let error = response.error.expect("error object");
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
    }

    #[tokio::test]
    async fn engine_namespace_has_placeholder_method_responses() {
        let new_payload = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_newPayloadV3"),
            &["Bearer dev-jwt"],
        )
        .await;
        assert!(new_payload.error.is_none());
        assert_eq!(
            new_payload.result.expect("result"),
            json!({
                "status": "SYNCING",
                "latestValidHash": null,
                "validationError": "Placeholder execution path in scaffold"
            })
        );

        let forkchoice = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_forkchoiceUpdatedV3"),
            &["Bearer dev-jwt"],
        )
        .await;
        assert!(forkchoice.error.is_none());
        assert_eq!(
            forkchoice.result.expect("result"),
            json!({
                "payloadStatus": {
                    "status": "SYNCING",
                    "latestValidHash": null,
                    "validationError": "Placeholder execution path in scaffold"
                },
                "payloadId": null
            })
        );

        let get_payload = call_json_rpc(
            RpcServerState::default(),
            "/engine",
            rpc_request("engine_getPayloadV3"),
            &["Bearer dev-jwt"],
        )
        .await;
        let get_payload_error = get_payload.error.expect("error");
        assert_eq!(get_payload_error.code, ENGINE_NOT_IMPLEMENTED_CODE);
        assert_eq!(
            get_payload_error.message,
            "Engine method placeholder not implemented"
        );
    }

    #[tokio::test]
    async fn engine_api_fails_closed_if_jwt_is_not_configured() {
        let response = call_json_rpc(
            RpcServerState {
                engine_jwt: " ".to_string(),
                ..RpcServerState::default()
            },
            "/engine",
            rpc_request("engine_exchangeCapabilities"),
            &["Bearer dev-jwt"],
        )
        .await;
        let error = response.error.expect("error");
        assert_eq!(error.code, ENGINE_AUTH_ERROR_CODE);
        assert_eq!(
            error.data.expect("reason"),
            json!({ "reason": "missing_configured_jwt" })
        );
    }
}
