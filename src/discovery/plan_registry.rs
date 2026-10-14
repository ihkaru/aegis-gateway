// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use crate::discovery::planner::ExecutionPlan;

/// In-memory thread-safe registry for generated execution plans with TTL eviction
#[derive(Clone)]
pub struct PlanRegistry {
    plans: Arc<RwLock<HashMap<String, (ExecutionPlan, Instant)>>>,
    ttl: Duration,
}

static SHARED_PLAN_REGISTRY: OnceLock<PlanRegistry> = OnceLock::new();

pub fn get_shared_plan_registry() -> &'static PlanRegistry {
    SHARED_PLAN_REGISTRY.get_or_init(|| PlanRegistry::new(Duration::from_secs(3600)))
}

impl PlanRegistry {
    pub fn new(ttl: Duration) -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub fn store_plan(&self, plan: ExecutionPlan) {
        if let Ok(mut lock) = self.plans.write() {
            // Evict expired entries if size exceeds limit
            if lock.len() > 200 {
                let now = Instant::now();
                lock.retain(|_, (_, ts)| now.duration_since(*ts) < self.ttl);
            }
            lock.insert(plan.plan_id.clone(), (plan, Instant::now()));
        }
    }

    pub fn get_plan(&self, plan_id: &str) -> Option<ExecutionPlan> {
        let lock = self.plans.read().ok()?;
        if let Some((plan, ts)) = lock.get(plan_id) {
            if Instant::now().duration_since(*ts) < self.ttl {
                return Some(plan.clone());
            }
        }
        None
    }

    pub fn clear(&self) {
        if let Ok(mut lock) = self.plans.write() {
            lock.clear();
        }
    }
}
