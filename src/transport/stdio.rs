// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::core::error::AegisResult;
use crate::core::transport::{IngressTransport, WireProtocolHandler};

/// Stdio client ingress transport with strict zero-contamination stdout channel
pub struct StdioTransport {
    handler: Arc<dyn WireProtocolHandler>,
}

impl StdioTransport {
    pub fn new(handler: Arc<dyn WireProtocolHandler>) -> Self {
        Self { handler }
    }

    /// Process a stream from arbitrary reader/writer pair (enables clean unit & integration testing)
    pub async fn process_stream<R, W>(&self, mut reader: R, mut writer: W) -> AegisResult<()>
    where
        R: AsyncBufRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Some(response_str) = self.handler.handle_message(trimmed).await? {
                    writer.write_all(response_str.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }
            line.clear();
        }

        Ok(())
    }
}

#[async_trait]
impl IngressTransport for StdioTransport {
    async fn run(&self) -> AegisResult<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let writer = stdout;

        eprintln!("[AEGIS] Starting Stdio client ingress (Claude Desktop / Cursor mode)...");
        self.process_stream(reader, writer).await
    }
}
