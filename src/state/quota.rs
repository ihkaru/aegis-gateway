// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::state::{
    BudgetManager, BudgetStatus, DepartmentChargeback, QuotaEngine, TenantBudget,
};
use crate::core::types::TenantId;

/// Real-time multi-tenant FinOps budget quota tracker with hard cutoff freeze
#[derive(Clone, Default)]
pub struct HardFreezeQuota {
    budgets: Arc<RwLock<HashMap<TenantId, TenantBudget>>>,
    default_monthly_limit: f64,
}

impl HardFreezeQuota {
    pub fn new() -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
            default_monthly_limit: 500.0,
        }
    }

    pub fn with_default_limit(limit_usd: f64) -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
            default_monthly_limit: limit_usd,
        }
    }
}

#[async_trait]
impl QuotaEngine for HardFreezeQuota {
    async fn check_budget(&self, tenant: &TenantId) -> AegisResult<bool> {
        let read = self.budgets.read().await;
        if let Some(budget) = read.get(tenant) {
            if budget.hard_freeze_enabled && budget.current_spend_usd >= budget.monthly_limit_usd {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> AegisResult<()> {
        let mut write = self.budgets.write().await;
        let entry = write.entry(tenant.clone()).or_insert_with(|| TenantBudget {
            tenant_id: tenant.clone(),
            department: "Unallocated".to_string(),
            monthly_limit_usd: self.default_monthly_limit,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        });

        if entry.hard_freeze_enabled && entry.current_spend_usd >= entry.monthly_limit_usd {
            return Err(AegisError::BudgetFrozen {
                tenant: tenant.as_str().to_string(),
                reason: format!(
                    "Spend limit ${:.2} exhausted (current: ${:.2})",
                    entry.monthly_limit_usd, entry.current_spend_usd
                ),
            });
        }

        entry.current_spend_usd += cost_usd;
        Ok(())
    }
}

#[async_trait]
impl BudgetManager for HardFreezeQuota {
    async fn set_budget(&self, budget: TenantBudget) -> AegisResult<()> {
        let mut write = self.budgets.write().await;
        write.insert(budget.tenant_id.clone(), budget);
        Ok(())
    }

    async fn get_budget(&self, tenant: &TenantId) -> AegisResult<Option<TenantBudget>> {
        let read = self.budgets.read().await;
        Ok(read.get(tenant).cloned())
    }

    async fn record_usage(&self, tenant: &TenantId, tokens: u64, cost_usd: f64) -> AegisResult<()> {
        let mut write = self.budgets.write().await;
        let entry = write.entry(tenant.clone()).or_insert_with(|| TenantBudget {
            tenant_id: tenant.clone(),
            department: "Unallocated".to_string(),
            monthly_limit_usd: self.default_monthly_limit,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        });

        if entry.hard_freeze_enabled && entry.current_spend_usd >= entry.monthly_limit_usd {
            return Err(AegisError::BudgetFrozen {
                tenant: tenant.as_str().to_string(),
                reason: format!(
                    "Hard budget cutoff active at ${:.2} (exhausted)",
                    entry.monthly_limit_usd
                ),
            });
        }

        entry.tokens_consumed += tokens;
        entry.current_spend_usd += cost_usd;
        Ok(())
    }

    async fn evaluate_status(&self, tenant: &TenantId) -> AegisResult<BudgetStatus> {
        let read = self.budgets.read().await;
        let budget = match read.get(tenant) {
            Some(b) => b,
            None => return Ok(BudgetStatus::Normal { percent_used: 0.0 }),
        };

        if budget.monthly_limit_usd <= 0.0 {
            return Ok(BudgetStatus::Normal { percent_used: 0.0 });
        }

        let percent = (budget.current_spend_usd / budget.monthly_limit_usd) * 100.0;
        if percent >= 100.0 {
            Ok(BudgetStatus::Frozen {
                percent_used: percent,
                reason: format!(
                    "Budget exhausted: ${:.2} / ${:.2}",
                    budget.current_spend_usd, budget.monthly_limit_usd
                ),
            })
        } else if percent >= 80.0 {
            Ok(BudgetStatus::Warning {
                percent_used: percent,
                message: format!(
                    "Warning: 80% threshold reached (${:.2} / ${:.2})",
                    budget.current_spend_usd, budget.monthly_limit_usd
                ),
            })
        } else {
            Ok(BudgetStatus::Normal { percent_used: percent })
        }
    }

    async fn generate_chargeback_report(&self) -> AegisResult<Vec<DepartmentChargeback>> {
        let read = self.budgets.read().await;
        let mut dept_map: HashMap<String, (f64, u64, usize)> = HashMap::new();

        for budget in read.values() {
            let entry = dept_map.entry(budget.department.clone()).or_insert((0.0, 0, 0));
            entry.0 += budget.current_spend_usd;
            entry.1 += budget.tokens_consumed;
            entry.2 += 1;
        }

        let mut report: Vec<DepartmentChargeback> = dept_map
            .into_iter()
            .map(|(dept, (cost, tokens, count))| DepartmentChargeback {
                department: dept,
                total_cost_usd: cost,
                total_tokens: tokens,
                tenant_count: count,
            })
            .collect();

        report.sort_by(|a, b| b.total_cost_usd.partial_cmp(&a.total_cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        Ok(report)
    }

    async fn render_prometheus_metrics(&self) -> AegisResult<String> {
        let read = self.budgets.read().await;
        let mut lines = Vec::new();

        lines.push("# HELP aegis_tenant_cost_usd_total Total cost incurred per tenant in USD".to_string());
        lines.push("# TYPE aegis_tenant_cost_usd_total counter".to_string());
        for (tenant, budget) in read.iter() {
            lines.push(format!(
                "aegis_tenant_cost_usd_total{{tenant=\"{}\",department=\"{}\"}} {:.4}",
                tenant.as_str(),
                budget.department,
                budget.current_spend_usd
            ));
        }

        lines.push("# HELP aegis_tenant_tokens_total Total tokens consumed per tenant".to_string());
        lines.push("# TYPE aegis_tenant_tokens_total counter".to_string());
        for (tenant, budget) in read.iter() {
            lines.push(format!(
                "aegis_tenant_tokens_total{{tenant=\"{}\",department=\"{}\"}} {}",
                tenant.as_str(),
                budget.department,
                budget.tokens_consumed
            ));
        }

        lines.push("# HELP aegis_tenant_budget_frozen 1 if budget is frozen, 0 otherwise".to_string());
        lines.push("# TYPE aegis_tenant_budget_frozen gauge".to_string());
        for (tenant, budget) in read.iter() {
            let is_frozen = if budget.hard_freeze_enabled && budget.current_spend_usd >= budget.monthly_limit_usd {
                1
            } else {
                0
            };
            lines.push(format!(
                "aegis_tenant_budget_frozen{{tenant=\"{}\",department=\"{}\"}} {}",
                tenant.as_str(),
                budget.department,
                is_frozen
            ));
        }

        Ok(lines.join("\n"))
    }
}
