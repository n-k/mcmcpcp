use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use crate::mcp::jsonrpc::{RpcMessage, RpcRequest};
use crate::mcp::transport::{InboundLine, StdioTransport};

#[derive(Debug, Clone)]
pub struct ServerSpec {
    pub id: String,
    pub cmd: String,
    pub args: Vec<String>,
}

pub struct McpServer {
    pub spec: ServerSpec,
    transport: Mutex<StdioTransport>,
    pending: Mutex<HashMap<String, tokio::sync::oneshot::Sender<RpcMessage>>>,
    pub tool_cache: Mutex<Vec<Value>>, // cache of tools/list
    req_timeout: Duration,
}

impl McpServer {
    pub async fn spawn(spec: ServerSpec, req_timeout: Duration, startup_timeout: Duration) -> Result<Self> {
        let mut child = Command::new(&spec.cmd)
            .args(&spec.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("spawning {}", spec.id))?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;

        let transport = StdioTransport::new(stdout, stderr, stdin);
        let server = Self {
            spec,
            transport: Mutex::new(transport),
            pending: Mutex::new(HashMap::new()),
            tool_cache: Mutex::new(vec![]),
            req_timeout,
        };

        // Spawn reader for stdout/stderr lines -> route responses
        server.start_reader();

        // Initialize handshake
        let _ = timeout(startup_timeout, server.initialize()).await
            .context("timeout waiting initialize")??;

        // Prefetch tools
        let _ = server.refresh_tools().await;

        Ok(server)
    }

    fn start_reader(&self) {
        let rx = self.transport.lock().rx_lines.clone();
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut rx = rx.expect("rx_lines present when starting reader");
            while let Some(line) = rx.recv().await {
                match line {
                    InboundLine::Stdout(s) => {
                        if let Ok(msg) = serde_json::from_str::<RpcMessage>(&s) {
                            // Route by id to pending waiter (if any)
                            let id = match &msg {
                                RpcMessage::Req(r) => r.id.clone(),
                                RpcMessage::Ok(r) => r.id.clone(),
                                RpcMessage::Err(r) => r.id.clone(),
                            }.to_string();

                            if let Some(tx) = pending.lock().remove(&id) {
                                let _ = tx.send(msg);
                            }
                        } else {
                            // Non-JSON noise from server; ignore or log
                            tracing::debug!(line=%s, "server stdout (non-json)");
                        }
                    }
                    InboundLine::Stderr(s) => {
                        tracing::warn!(line=%s, "server stderr");
                    }
                }
            }
        });
    }

    async fn initialize(&self) -> Result<Value> {
        self.rpc_call("initialize", json!({
            "clientName": "rust-mcp-host",
            "clientVersion": "0.1.0",
        })).await
    }

    pub async fn refresh_tools(&self) -> Result<()> {
        let tools = self.rpc_call("tools/list", Value::Null).await?;
        // tools result shape is server-dependent; store opaque JSON
        *self.tool_cache.lock() = vec![tools];
        Ok(())
    }

    pub async fn rpc_call(&self, method: &str, params: Value) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().insert(id.clone(), tx);

        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Value::String(id.clone()),
            method: method.into(),
            params: if params.is_null() { None } else { Some(params) },
        };
        let v = serde_json::to_value(&req)?;
        self.transport.lock().send_json(&v).await?;

        let msg = timeout(self.req_timeout, rx).await
            .map_err(|_| anyhow!("rpc {} timed out", method))?
            .map_err(|_| anyhow!("rpc {} channel closed", method))?;

        match msg {
            RpcMessage::Ok(ok) => Ok(ok.result),
            RpcMessage::Err(e) => Err(anyhow!("rpc error {}: {} {:?}", method, e.error.message, e.error.data)),
            RpcMessage::Req(_r) => Err(anyhow!("unexpected request from server during call")),
        }
    }
}
