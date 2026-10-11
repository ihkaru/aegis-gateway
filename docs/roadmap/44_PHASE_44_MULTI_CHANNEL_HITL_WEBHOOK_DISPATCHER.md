# Phase 44: Multi-Channel HITL Webhook Dispatcher (Slack BlockKit & Teams Cards)

> **Milestone Tag**: `v1.44.0-multi_channel_hitl_webhook_dispatcher`  
> **Status**: `In Planning & Implementation`  
> **Target Enterprise Gaps & Standards**:
> - Delivers real-time interactive approval requests for suspended high-risk agent operations to corporate messaging platforms (Slack & Microsoft Teams).
> - Cryptographic Attestation: Every webhook payload includes `X-Aegis-Signature` (HMAC-SHA256) and `X-Aegis-Timestamp` to prevent spoofing or replay attacks.
> - Standard Compliance: NIST SP 800-53 Rev. 5 (AC-2/AC-3), SOC 2 Trust Services Criteria CC6.1.

---

## 1. Problem Statement & Motivation

Prior implementations tracked suspended actions in in-memory queues and the embedded Web UI, but lacked:
1. Automated outward notification dispatchers to IT operations channels where managers actually work.
2. Structured message formatting for Slack BlockKit (interactive blocks with action buttons) and Microsoft Teams Adaptive Cards.
3. Cryptographic payload signing so receiving endpoints can verify that the approval card originated from the verified Aegis Gateway instance.

---

## 2. Technical Architecture & Trait Contracts

```rust
#[async_trait]
pub trait HitlWebhookDispatcher: Send + Sync {
    async fn dispatch_approval_request(
        &self,
        request: &ApprovalWebhookPayload,
    ) -> AegisResult<WebhookDispatchResult>;
}
```

### Key Guarantees:
- **Slack BlockKit Format**: Generates canonical Slack blocks with summary, risk tier, action buttons (`Approve`, `Reject`), and metadata.
- **Teams Adaptive Card Format**: Generates Adaptive Card JSON schema 1.5 with action sets.
- **HMAC Signatures**: Signs outgoing payloads using gateway HMAC secret key.
- **Zero Mock**: Real JSON generation, real crypto HMAC computation, real HTTP dispatching capability.
