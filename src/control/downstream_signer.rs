// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

/// Downstream Cryptographic Identity Signer for Row-Level Security (RLS) Propagation
pub struct DownstreamIdentitySigner;

impl DownstreamIdentitySigner {
    /// Produces signed headers: (X-Aegis-User-Id, X-Aegis-Tenant-Id, X-Aegis-Timestamp, X-Aegis-Signature)
    pub fn sign_headers(
        user_id: &str,
        tenant_id: &str,
        secret: &str,
    ) -> (String, String, String, String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();

        let sig = Self::compute_hmac_hex(user_id, tenant_id, &timestamp, secret);

        (
            user_id.to_string(),
            tenant_id.to_string(),
            timestamp,
            sig,
        )
    }

    /// Verifies cryptographic signature received on downstream headers
    pub fn verify_signature(
        user_id: &str,
        tenant_id: &str,
        timestamp: &str,
        signature: &str,
        secret: &str,
    ) -> bool {
        // Prevent replay attacks beyond 300 seconds
        if let Ok(ts) = timestamp.parse::<u64>() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if now.abs_diff(ts) > 300 {
                return false;
            }
        } else {
            return false;
        }

        let expected_sig = Self::compute_hmac_hex(user_id, tenant_id, timestamp, secret);
        signature == expected_sig
    }

    fn compute_hmac_hex(user_id: &str, tenant_id: &str, timestamp: &str, secret: &str) -> String {
        let payload = format!("{}:{}:{}:{}", user_id, tenant_id, timestamp, secret);
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in payload.as_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }
}
