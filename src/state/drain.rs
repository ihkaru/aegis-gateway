// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use crate::core::error::{AegisError, AegisResult};

/// RAII guard releasing an inflight task slot on drop
pub struct TaskGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Graceful Shutdown Coordinator managing inflight tool call draining
pub struct DrainCoordinator {
    active_tasks: Arc<AtomicUsize>,
    draining: Arc<AtomicBool>,
}

impl DrainCoordinator {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(AtomicUsize::new(0)),
            draining: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Try to acquire a slot for executing a tool call
    pub fn acquire_slot(&self) -> AegisResult<TaskGuard> {
        if self.draining.load(Ordering::SeqCst) {
            return Err(AegisError::DrainTimeout(
                "Gateway is in graceful shutdown; new tool requests rejected".to_string(),
            ));
        }
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        Ok(TaskGuard {
            counter: Arc::clone(&self.active_tasks),
        })
    }

    /// Signal shutdown (SIGTERM/SIGINT) to reject incoming requests and drain inflight
    pub fn signal_shutdown(&self) {
        self.draining.store(true, Ordering::SeqCst);
    }

    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    pub fn inflight_count(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }

    /// Wait for all inflight tasks to drain within the specified timeout
    pub async fn wait_drain(&self, timeout: Duration) -> AegisResult<()> {
        self.signal_shutdown();
        let deadline = tokio::time::Instant::now() + timeout;

        while self.active_tasks.load(Ordering::SeqCst) > 0 {
            if tokio::time::Instant::now() >= deadline {
                let remaining = self.active_tasks.load(Ordering::SeqCst);
                return Err(AegisError::DrainTimeout(format!(
                    "Timed out draining {} inflight tasks after {:?}",
                    remaining, timeout
                )));
            }
            sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }
}

impl Default for DrainCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
