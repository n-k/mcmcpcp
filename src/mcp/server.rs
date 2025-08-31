use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use dioxus::logger::tracing::{debug, warn};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::mcp::host::_Server;
use crate::mcp::jsonrpc::{RpcMessage, RpcRequest};
use crate::mcp::{McpTool, ServerSpec};

pub struct McpServer {
    #[allow(unused)]
    pub spec: ServerSpec,
    #[cfg(not(target_arch = "wasm32"))]
    transport: Mutex<crate::mcp::transport::StdioTransport>,
    pending: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<RpcMessage>>>>,
    pub tool_cache: Mutex<Vec<McpTool>>,
    req_timeout: Duration,
    count: Mutex<u32>,
}

#[async_trait::async_trait]
impl _Server for McpServer {
    async fn list_tools(&self) -> Vec<McpTool> {
        self.tool_cache.lock().await.clone()
    }

    async fn rpc(
        &self, 
        method: &str, 
        params: Value
    ) -> anyhow::Result<serde_json::Value> {
        self.rpc_call(method, params).await
    }
}

impl McpServer {
    #[cfg(target_arch = "wasm32")]
    pub async fn spawn(
        spec: ServerSpec,
        req_timeout: Duration,
        _startup_timeout: Duration,
    ) -> Result<Self> {
        Ok(Self {
            spec,
            pending: Arc::new(Mutex::new(HashMap::new())),
            tool_cache: Mutex::new(vec![]),
            req_timeout,
            count: Mutex::new(0),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn spawn(
        spec: ServerSpec,
        req_timeout: Duration,
        startup_timeout: Duration,
    ) -> Result<Self> {
        use tokio::time::timeout;
        use crate::mcp::transport::StdioTransport;

        let mut child = tokio::process::Command::new(&spec.cmd)
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
            count: Mutex::new(0),
        };

        // Spawn reader for stdout/stderr lines -> route responses
        server.start_reader().await;

        // Initialize handshake
        timeout(startup_timeout, server.initialize())
            .await
            .context("timeout waiting initialize")??;
        // Prefetch tools
        server.refresh_tools().await?;

        Ok(server)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn start_reader(&self) {
        let rx = self.transport.lock().await.rx_lines.take();
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut rx = rx.expect("rx_lines present when starting reader");
            while let Some(line) = rx.recv().await {
                match line {
                    crate::mcp::transport::InboundLine::Stdout(s) => {
                        let msg = serde_json::from_str::<RpcMessage>(&s);
                        if let Ok(msg) = msg {
                            // Route by id to pending waiter (if any)
                            let id = match &msg {
                                RpcMessage::Req(r) => r.id.clone(),
                                RpcMessage::Ok(r) => r.id.clone(),
                                RpcMessage::Err(r) => r.id.clone(),
                            }
                            .clone();
                            let id = id.as_str().unwrap_or_else(|| "");

                            if let Some(tx) = pending.lock().await.remove(id) {
                                if let Err(e) = tx.send(msg) {
                                    eprintln!("Error sending to oneshot: {e:?}");
                                }
                            }
                        } else {
                            // Non-JSON noise from server; ignore or log
                            debug!(line=%s, "server stdout (non-json)");
                        }
                    }
                    crate::mcp::transport::InboundLine::Stderr(s) => {
                        warn!(line=%s, "server stderr");
                    }
                }
            }
        });
    }

    async fn initialize(&self) -> Result<Value> {
        self.rpc_call(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "mcmcpcp",
                    "version": "1",
                },
                "capabilities": {},
            }),
        )
        .await
    }

    pub async fn refresh_tools(&self) -> Result<()> {
        let tools = self.rpc_call("tools/list", json!({})).await?;
        let tools: Vec<McpTool> =
            serde_json::from_value(tools.get("tools").cloned().unwrap_or_default())?;
        *self.tool_cache.lock().await = tools;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn rpc_call(&self, _method: &str, _params: Value) -> Result<Value> {
        Ok(Value::Null)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn rpc_call(&self, method: &str, params: Value) -> Result<Value> {
        use tokio::time::timeout;

        let id = {
            let mut l = self.count.lock().await;
            let c = *l;
            *l = c + 1;
            c
        };
        let id = format!("{id}");
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);

        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Value::String(id),
            method: method.into(),
            params: if params.is_null() { None } else { Some(params) },
        };
        let v = serde_json::to_value(&req)?;
        self.transport.lock().await.send_json(&v).await?;

        let msg = timeout(self.req_timeout, rx)
            .await
            .map_err(|_| anyhow!("rpc {} timed out", method))?
            .map_err(|_| anyhow!("rpc {} channel closed", method))?;

        match msg {
            RpcMessage::Ok(ok) => Ok(ok.result),
            RpcMessage::Err(e) => Err(anyhow!(
                "rpc error {}: {} {:?}",
                method,
                e.error.message,
                e.error.data
            )),
            RpcMessage::Req(_r) => Err(anyhow!("unexpected request from server during call")),
        }
    }
}
