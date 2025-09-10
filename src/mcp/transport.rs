// Copyright © 2025 Nipun Kumar

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::ChildStdin,
};

#[derive(Debug)]
pub enum InboundLine {
    Stdout(String),
    Stderr(String),
}

pub struct StdioTransport {
    stdin: ChildStdin,
    pub rx_lines: Option<mpsc::UnboundedReceiver<InboundLine>>,
}

impl StdioTransport {
    pub fn new(
        stdout: ChildStdout,
        stderr: tokio::process::ChildStderr,
        stdin: ChildStdin,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // stdout reader
        let mut out_reader = BufReader::new(stdout).lines();
        let tx_out = tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = out_reader.next_line().await {
                let _ = tx_out.send(InboundLine::Stdout(line));
            }
        });

        // stderr reader
        let mut err_reader = BufReader::new(stderr).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = err_reader.next_line().await {
                let _ = tx.send(InboundLine::Stderr(line));
            }
        });

        Self {
            stdin,
            rx_lines: Some(rx),
        }
    }

    pub async fn send_json(&mut self, v: &Value) -> Result<()> {
        let mut s = serde_json::to_string(v)?;
        s.push('\n');
        self.stdin
            .write_all(s.as_bytes())
            .await
            .context("writing to child stdin")?;
        self.stdin.flush().await?;
        Ok(())
    }
}
