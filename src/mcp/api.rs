use std::sync::Arc;
use axum::{extract::{State, WebSocketUpgrade}, routing::{get, post}, Json, Router};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::host::Host;

#[derive(Clone)]
pub struct AppState {
    pub host: Arc<Host>,
}

#[derive(Serialize)]
pub struct ToolsResponse {
    tools: Vec<crate::host::ToolDescriptor>,
}

#[derive(Deserialize)]
pub struct InvokeBody {
    pub server_id: String,
    pub method: String,
    pub params: Option<Value>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/tools", get(get_tools))
        .route("/invoke", post(post_invoke))
        .route("/ws/logs", get(ws_logs)) // optional demo
        .with_state(state)
}

async fn get_tools(State(state): State<AppState>) -> impl IntoResponse {
    let tools = state.host.list_tools();
    Json(ToolsResponse { tools })
}

async fn post_invoke(State(state): State<AppState>, Json(body): Json<InvokeBody>) -> impl IntoResponse {
    match state.host.invoke(&body.server_id, &body.method, body.params.unwrap_or(Value::Null)).await {
        Ok(v) => Json(json!({ "ok": true, "result": v })).into_response(),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })).into_response(),
    }
}

use axum::extract::ws::{Message, WebSocket};
async fn ws_logs(ws: WebSocketUpgrade, State(_state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        // If you want: push logs, heartbeats, etc.
        let _ = socket.send(Message::Text("connected".into())).await;
    })
}
