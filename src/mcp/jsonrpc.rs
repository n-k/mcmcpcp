use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Value,               // allow string or number
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcSuccess {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcErrorObj {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: RpcErrorObj,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RpcMessage {
    Req(RpcRequest),
    Ok(RpcSuccess),
    Err(RpcError),
}

pub fn req(method: &str, id: Value, params: Option<Value>) -> RpcRequest {
    RpcRequest {
        jsonrpc: "2.0".into(),
        id,
        method: method.into(),
        params,
    }
}
