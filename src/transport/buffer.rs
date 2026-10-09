//! Reusable Stream Buffer for MCP Streamable Ingress
//! Resolves microsoft/mcp-gateway#48 ("The stream was already consumed. It cannot be read again.")
//! Enables multi-pass reading across DLP, ABAC policy, and backend proxies without stream exhaustion.

use crate::core::error::AegisError;
use serde_json::Value;
use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ReusableStreamBuffer {
    data: Arc<[u8]>,
    max_capacity: usize,
}

impl ReusableStreamBuffer {
    pub const DEFAULT_MAX_CAPACITY: usize = 16 * 1024 * 1024; // 16 MB

    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        let vec: Vec<u8> = data.into();
        Self {
            data: Arc::from(vec),
            max_capacity: Self::DEFAULT_MAX_CAPACITY,
        }
    }

    pub fn with_max_capacity(data: impl Into<Vec<u8>>, max_capacity: usize) -> Result<Self, AegisError> {
        let vec: Vec<u8> = data.into();
        if vec.len() > max_capacity {
            return Err(AegisError::StateError(format!(
                "Payload size {} exceeds maximum buffer capacity {}",
                vec.len(),
                max_capacity
            )));
        }
        Ok(Self {
            data: Arc::from(vec),
            max_capacity,
        })
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Fork an independent reader cursor at byte position 0
    pub fn fork_reader(&self) -> Cursor<Arc<[u8]>> {
        Cursor::new(Arc::clone(&self.data))
    }

    /// Parse JSON payload safely multiple times without stream exhaustion
    pub fn read_json(&self) -> Result<Value, AegisError> {
        if self.data.is_empty() {
            return Err(AegisError::StateError("Cannot parse empty buffer as JSON".to_string()));
        }
        serde_json::from_slice(&self.data).map_err(|e| {
            AegisError::StateError(format!("Malformed JSON in reusable stream buffer: {}", e))
        })
    }

    /// Read buffer contents as a UTF-8 string without consuming
    pub fn read_utf8(&self) -> Result<String, AegisError> {
        String::from_utf8(self.data.to_vec()).map_err(|e| {
            AegisError::StateError(format!("Invalid UTF-8 in stream buffer: {}", e))
        })
    }

    /// Simulate multiple consumer passes over the stream
    pub fn multi_pass_consume<T, F>(&self, passes: usize, mut consumer: F) -> Result<Vec<T>, AegisError>
    where
        F: FnMut(&mut Cursor<Arc<[u8]>>, usize) -> Result<T, AegisError>,
    {
        let mut results = Vec::with_capacity(passes);
        for pass_idx in 0..passes {
            let mut cursor = self.fork_reader();
            let res = consumer(&mut cursor, pass_idx)?;
            results.push(res);
        }
        Ok(results)
    }
}
