// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use chrono::Utc;

use crate::core::secrets::SecretStore;
use super::types::{MaskedKeyInfo, SelectionStrategy};

/// Representation of an API key inside the managed pool
#[derive(Debug, Clone)]
pub struct PooledKey {
    pub id: String,
    pub raw_key: String,
    pub label: String,
    pub active: bool,
    pub usage_count: u64,
    pub failure_count: u32,
    pub last_used_at: Option<chrono::DateTime<Utc>>,
    pub cooldown_until: Option<Instant>,
}

impl PooledKey {
    pub fn new(id: impl Into<String>, raw_key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            raw_key: raw_key.into(),
            label: label.into(),
            active: true,
            usage_count: 0,
            failure_count: 0,
            last_used_at: None,
            cooldown_until: None,
        }
    }

    pub fn is_available(&self, now: Instant) -> bool {
        if !self.active {
            return false;
        }
        match self.cooldown_until {
            Some(expiry) => now >= expiry,
            None => true,
        }
    }

    pub fn mask(&self) -> String {
        mask_secret(&self.raw_key)
    }

    pub fn to_info(&self, now: Instant) -> MaskedKeyInfo {
        let in_cooldown = match self.cooldown_until {
            Some(expiry) => now < expiry,
            None => false,
        };
        MaskedKeyInfo {
            id: self.id.clone(),
            masked_key: self.mask(),
            label: self.label.clone(),
            active: self.active,
            usage_count: self.usage_count,
            failure_count: self.failure_count,
            last_used_at: self.last_used_at.map(|dt| dt.to_rfc3339()),
            in_cooldown,
        }
    }
}

pub fn mask_secret(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= 8 {
        return "********".to_string();
    }
    format!("{}...{}", &trimmed[..4], &trimmed[trimmed.len() - 4..])
}

/// Dynamic Key Pool managing round-robin, least-used routing and rate-limit cooldown
pub struct SearchKeyPool {
    keys: Vec<PooledKey>,
    rr_cursor: AtomicUsize,
    strategy: SelectionStrategy,
}

impl SearchKeyPool {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            rr_cursor: AtomicUsize::new(0),
            strategy: SelectionStrategy::RoundRobin,
        }
    }

    pub fn set_strategy(&mut self, strategy: SelectionStrategy) {
        self.strategy = strategy;
    }

    pub fn strategy(&self) -> SelectionStrategy {
        self.strategy
    }

    pub fn add_key(&mut self, raw_key: impl Into<String>, label: Option<String>) -> String {
        let key_str = raw_key.into().trim().to_string();
        if let Some(existing) = self.keys.iter().find(|k| k.raw_key == key_str) {
            return existing.id.clone();
        }
        let id = format!("key-{}", uuid::Uuid::new_v4().simple());
        let final_label = label.unwrap_or_else(|| format!("Key #{}", self.keys.len() + 1));
        self.keys.push(PooledKey::new(id.clone(), key_str, final_label));
        id
    }

    pub fn remove_key(&mut self, id: &str) -> bool {
        let initial_len = self.keys.len();
        self.keys.retain(|k| k.id != id);
        self.keys.len() < initial_len
    }

    pub fn toggle_key(&mut self, id: &str, active: bool) -> bool {
        if let Some(k) = self.keys.iter_mut().find(|k| k.id == id) {
            k.active = active;
            true
        } else {
            false
        }
    }

    pub fn list_keys(&self) -> Vec<MaskedKeyInfo> {
        let now = Instant::now();
        self.keys.iter().map(|k| k.to_info(now)).collect()
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    pub fn available_key_count(&self) -> usize {
        let now = Instant::now();
        self.keys.iter().filter(|k| k.is_available(now)).count()
    }

    /// Select next available key for execution based on active strategy
    pub fn acquire_key(&self) -> Option<(String, String)> {
        let now = Instant::now();
        let available_indices: Vec<usize> = self
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.is_available(now))
            .map(|(i, _)| i)
            .collect();

        if available_indices.is_empty() {
            return None;
        }

        match self.strategy {
            SelectionStrategy::RoundRobin => {
                let current = self.rr_cursor.fetch_add(1, Ordering::Relaxed);
                let selected_idx = available_indices[current % available_indices.len()];
                let k = &self.keys[selected_idx];
                Some((k.id.clone(), k.raw_key.clone()))
            }
            SelectionStrategy::LeastUsed => {
                let mut best_idx = available_indices[0];
                let mut min_usage = self.keys[best_idx].usage_count;
                for &idx in &available_indices[1..] {
                    if self.keys[idx].usage_count < min_usage {
                        min_usage = self.keys[idx].usage_count;
                        best_idx = idx;
                    }
                }
                let k = &self.keys[best_idx];
                Some((k.id.clone(), k.raw_key.clone()))
            }
        }
    }

    pub fn record_success(&mut self, key_id: &str) {
        if let Some(k) = self.keys.iter_mut().find(|k| k.id == key_id) {
            k.usage_count += 1;
            k.last_used_at = Some(Utc::now());
        }
    }

    pub fn record_rate_limit(&mut self, key_id: &str, cooldown: Duration) {
        if let Some(k) = self.keys.iter_mut().find(|k| k.id == key_id) {
            k.failure_count += 1;
            k.cooldown_until = Some(Instant::now() + cooldown);
        }
    }

    pub fn record_unauthorized(&mut self, key_id: &str) {
        if let Some(k) = self.keys.iter_mut().find(|k| k.id == key_id) {
            k.failure_count += 1;
            k.active = false;
        }
    }

    /// Populate pool dynamically from Infisical / Environment store
    pub async fn load_from_secret_store(&mut self, store: &dyn SecretStore) -> usize {
        let mut loaded = 0;
        // 1. Numbered keys: EXA_API_KEY_1..10
        for i in 1..=10 {
            let key_name = format!("EXA_API_KEY_{i}");
            if let Ok(Some(val)) = store.get_secret(&key_name).await {
                if !val.trim().is_empty() {
                    let prev_count = self.keys.len();
                    self.add_key(val, Some(format!("Exa Key #{i}")));
                    if self.keys.len() > prev_count {
                        loaded += 1;
                    }
                }
            }
        }

        // 2. Comma-separated list: EXA_API_KEYS
        if let Ok(Some(list)) = store.get_secret("EXA_API_KEYS").await {
            for (idx, raw_token) in list.split(',').enumerate() {
                let token = raw_token.trim();
                if !token.is_empty() {
                    let prev_count = self.keys.len();
                    self.add_key(token, Some(format!("Exa Pooled Key #{}", idx + 1)));
                    if self.keys.len() > prev_count {
                        loaded += 1;
                    }
                }
            }
        }

        // 3. Fallback single key: EXA_API_KEY
        if let Ok(Some(single)) = store.get_secret("EXA_API_KEY").await {
            if !single.trim().is_empty() {
                let prev_count = self.keys.len();
                self.add_key(single, Some("Default Exa Key".to_string()));
                if self.keys.len() > prev_count {
                    loaded += 1;
                }
            }
        }

        loaded
    }
}

impl Default for SearchKeyPool {
    fn default() -> Self {
        Self::new()
    }
}
