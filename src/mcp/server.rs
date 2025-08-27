use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use dioxus::logger::tracing::{debug, warn};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

pub struct McpServer {
    #[allow(unused)]
    pub spec: ServerSpec,
    transport: Mutex<StdioTransport>,
    pending: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<RpcMessage>>>>,
    pub tool_cache: Mutex<Vec<Tool>>,
    req_timeout: Duration,
}

impl McpServer {
    pub async fn spawn(spec: ServerSpec,
        req_timeout: Duration,
        startup_timeout: Duration,
    ) -> Result<Self> {
        let mut child = Command::new(&spec.cmd)
            .args(&spec.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("spawning {}", spec.id))?;
        // tokio::time::sleep(Duration::from_secs(10)).await;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;

        let transport = StdioTransport::new(stdout, stderr, stdin);
        let server = Self {
            spec,
            transport: Mutex::new(transport),
            pending: Arc::new(Mutex::new(HashMap::new())),
            tool_cache: Mutex::new(vec![]),
            req_timeout,
        };

        // Spawn reader for stdout/stderr lines -> route responses
        server.start_reader();

        // Initialize handshake
        timeout(startup_timeout, server.initialize()).await
            .context("timeout waiting initialize")??;
        // Prefetch tools
        server.refresh_tools().await?;

        Ok(server)
    }

    fn start_reader(&self) {
        let rx = self.transport.lock().rx_lines.take();
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut rx = rx.expect("rx_lines present when starting reader");
            while let Some(line) = rx.recv().await {
                match line {
                    InboundLine::Stdout(s) => {
                        let msg = serde_json::from_str::<RpcMessage>(&s);
                        if let Ok(msg) = msg {
                            // Route by id to pending waiter (if any)
                            let id = match &msg {
                                RpcMessage::Req(r) => r.id.clone(),
                                RpcMessage::Ok(r) => r.id.clone(),
                                RpcMessage::Err(r) => r.id.clone(),
                            }.clone();
                            let id = id.as_str().unwrap_or_else(|| "");

                            if let Some(tx) = pending.lock().remove(id) {
                                if let Err(e) = tx.send(msg) {
                                    eprintln!("Error sending to oneshot: {e:?}");
                                }
                            }
                        } else {
                            // Non-JSON noise from server; ignore or log
                            debug!(line=%s, "server stdout (non-json)");
                        }
                    }
                    InboundLine::Stderr(s) => {
                        warn!(line=%s, "server stderr");
                    }
                }
            }
        });
    }

    async fn initialize(&self) -> Result<Value> {
        self.rpc_call("initialize", json!({
            "protocolVersion": "2025-06-18",
            "clientInfo": {
                "name": "mcmcpcp",
                "version": "1",
            },
            "capabilities": {},
        })).await
    }

    pub async fn refresh_tools(&self) -> Result<()> {
        let tools = self.rpc_call(
            "tools/list", 
            json!({})
        ).await?;
        let tools: Vec<Tool> = serde_json::from_value(tools.get("tools").cloned().unwrap_or_default())?;
        *self.tool_cache.lock() = tools;
        Ok(())
    }

    pub async fn rpc_call(&self, method: &str, params: Value) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().insert(id.clone(), tx);

        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Value::String(id),
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
