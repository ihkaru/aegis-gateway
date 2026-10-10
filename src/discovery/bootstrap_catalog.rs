// SPDX-License-Identifier: MIT

use serde_json::json;
use crate::core::types::ToolDefinition;

/// Provides foundational enterprise tool definitions for standalone discovery and planning
pub fn default_enterprise_catalog() -> Vec<ToolDefinition> {
    let mut tools = vec![
        // Database & Storage
        ToolDefinition {
            name: "query_postgresql_ledger".to_string(),
            server: "db_cluster".to_string(),
            description: "Execute SQL queries to retrieve transactional ledger entries".to_string(),
            input_schema: json!({ "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }),
            required_params: vec!["query".to_string()],
            when_to_use: "When financial, user, or tabular records need to be retrieved".to_string(),
            tags: vec!["database".to_string(), "sql".to_string(), "postgres".to_string(), "tabel".to_string(), "data".to_string()],
        },
        ToolDefinition {
            name: "destructive_drop_table".to_string(),
            server: "db_cluster".to_string(),
            description: "Permanently drop and delete database table and schema".to_string(),
            input_schema: json!({ "type": "object", "properties": { "table": { "type": "string" } }, "required": ["table"] }),
            required_params: vec!["table".to_string()],
            when_to_use: "When purging or discarding an entire database table".to_string(),
            tags: vec!["drop".to_string(), "delete".to_string(), "hapus".to_string(), "table".to_string(), "tabel".to_string()],
        },
        ToolDefinition {
            name: "upload_to_storage".to_string(),
            server: "mediavault".to_string(),
            description: "Store documents and images into persistent S3 cloud bucket".to_string(),
            input_schema: json!({ "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } }, "required": ["path"] }),
            required_params: vec!["path".to_string()],
            when_to_use: "When uploading or saving files, media, or archives into cloud storage".to_string(),
            tags: vec!["s3".to_string(), "file".to_string(), "berkas".to_string(), "upload".to_string(), "simpan".to_string(), "media".to_string()],
        },
        ToolDefinition {
            name: "delete_storage_object".to_string(),
            server: "mediavault".to_string(),
            description: "Permanently delete an uploaded document or image from S3".to_string(),
            input_schema: json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }),
            required_params: vec!["path".to_string()],
            when_to_use: "When discarding or removing files from persistent storage".to_string(),
            tags: vec!["s3".to_string(), "file".to_string(), "berkas".to_string(), "delete".to_string(), "hapus".to_string()],
        },
        ToolDefinition {
            name: "generate_presigned_url".to_string(),
            server: "mediavault".to_string(),
            description: "Create temporary presigned download link for storage object".to_string(),
            input_schema: json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }),
            required_params: vec!["path".to_string()],
            when_to_use: "When generating secure expirable links for clients to download files".to_string(),
            tags: vec!["s3".to_string(), "url".to_string(), "link".to_string(), "unduh".to_string(), "download".to_string()],
        },
        // Payments & Billing
        ToolDefinition {
            name: "stripe_create_customer".to_string(),
            server: "stripe_billing".to_string(),
            description: "Register new paying customer and initialize billing ledger".to_string(),
            input_schema: json!({ "type": "object", "properties": { "email": { "type": "string" } }, "required": ["email"] }),
            required_params: vec!["email".to_string()],
            when_to_use: "When onboarding a new account for commercial checkout".to_string(),
            tags: vec!["stripe".to_string(), "customer".to_string(), "pelanggan".to_string(), "billing".to_string()],
        },
        ToolDefinition {
            name: "stripe_charge_card".to_string(),
            server: "stripe_billing".to_string(),
            description: "Authorize and capture charge on customer credit card".to_string(),
            input_schema: json!({ "type": "object", "properties": { "customer_id": { "type": "string" }, "amount": { "type": "integer" } }, "required": ["customer_id", "amount"] }),
            required_params: vec!["customer_id".to_string(), "amount".to_string()],
            when_to_use: "When processing a real-time monetary payment transaction".to_string(),
            tags: vec!["stripe".to_string(), "charge".to_string(), "bayar".to_string(), "payment".to_string(), "kartu".to_string()],
        },
        ToolDefinition {
            name: "stripe_issue_refund".to_string(),
            server: "stripe_billing".to_string(),
            description: "Reverse settled payment transaction and return funds to buyer".to_string(),
            input_schema: json!({ "type": "object", "properties": { "charge_id": { "type": "string" } }, "required": ["charge_id"] }),
            required_params: vec!["charge_id".to_string()],
            when_to_use: "When customer requests dispute or payment reimbursement".to_string(),
            tags: vec!["stripe".to_string(), "refund".to_string(), "kembalikan".to_string(), "retur".to_string()],
        },
        ToolDefinition {
            name: "tax_calculate_vat".to_string(),
            server: "fiscal_calc".to_string(),
            description: "Calculate local VAT and sales tax percentage by jurisdiction".to_string(),
            input_schema: json!({ "type": "object", "properties": { "amount": { "type": "number" }, "country": { "type": "string" } }, "required": ["amount", "country"] }),
            required_params: vec!["amount".to_string(), "country".to_string()],
            when_to_use: "When applying legal sales tax before invoice dispatch".to_string(),
            tags: vec!["tax".to_string(), "pajak".to_string(), "vat".to_string(), "hitung".to_string()],
        },
        // Notifications & Messaging
        ToolDefinition {
            name: "dispatch_slack_alert".to_string(),
            server: "ops_notify".to_string(),
            description: "Broadcast incident alert message to engineering Slack channel".to_string(),
            input_schema: json!({ "type": "object", "properties": { "channel": { "type": "string" }, "message": { "type": "string" } }, "required": ["channel", "message"] }),
            required_params: vec!["channel".to_string(), "message".to_string()],
            when_to_use: "When notifying engineering teams about system events".to_string(),
            tags: vec!["slack".to_string(), "alert".to_string(), "pesan".to_string(), "kirim".to_string(), "notifikasi".to_string()],
        },
        ToolDefinition {
            name: "sendgrid_send_invoice_email".to_string(),
            server: "email_gateway".to_string(),
            description: "Dispatch PDF invoice email receipt to customer inbox".to_string(),
            input_schema: json!({ "type": "object", "properties": { "to": { "type": "string" }, "invoice_id": { "type": "string" } }, "required": ["to", "invoice_id"] }),
            required_params: vec!["to".to_string(), "invoice_id".to_string()],
            when_to_use: "When sending official receipts or invoices to customer mail".to_string(),
            tags: vec!["email".to_string(), "invoice".to_string(), "tagihan".to_string(), "kirim".to_string(), "surel".to_string()],
        },
        ToolDefinition {
            name: "twilio_send_sms_otp".to_string(),
            server: "sms_gateway".to_string(),
            description: "Send one-time SMS verification passcode to mobile number".to_string(),
            input_schema: json!({ "type": "object", "properties": { "phone": { "type": "string" } }, "required": ["phone"] }),
            required_params: vec!["phone".to_string()],
            when_to_use: "When dispatching two-factor authentication tokens over SMS".to_string(),
            tags: vec!["sms".to_string(), "otp".to_string(), "pesan".to_string(), "kirim".to_string(), "telepon".to_string()],
        },
        // Infrastructure & DevOps
        ToolDefinition {
            name: "docker_restart_container".to_string(),
            server: "devops_hub".to_string(),
            description: "Send SIGTERM and restart running containerized application".to_string(),
            input_schema: json!({ "type": "object", "properties": { "container_id": { "type": "string" } }, "required": ["container_id"] }),
            required_params: vec!["container_id".to_string()],
            when_to_use: "When restarting an unresponsive docker container".to_string(),
            tags: vec!["docker".to_string(), "container".to_string(), "restart".to_string(), "jalankan".to_string()],
        },
        ToolDefinition {
            name: "k8s_scale_deployment".to_string(),
            server: "devops_hub".to_string(),
            description: "Scale replica count for Kubernetes pod deployment".to_string(),
            input_schema: json!({ "type": "object", "properties": { "deployment": { "type": "string" }, "replicas": { "type": "integer" } }, "required": ["deployment", "replicas"] }),
            required_params: vec!["deployment".to_string(), "replicas".to_string()],
            when_to_use: "When scaling kubernetes cluster pods up or down".to_string(),
            tags: vec!["k8s".to_string(), "kubernetes".to_string(), "scale".to_string(), "pod".to_string()],
        },
        ToolDefinition {
            name: "github_create_pull_request".to_string(),
            server: "devops_hub".to_string(),
            description: "Create pull request for feature branch review".to_string(),
            input_schema: json!({ "type": "object", "properties": { "title": { "type": "string" }, "branch": { "type": "string" } }, "required": ["title", "branch"] }),
            required_params: vec!["title".to_string(), "branch".to_string()],
            when_to_use: "When opening a code review request on git repositories".to_string(),
            tags: vec!["github".to_string(), "git".to_string(), "pr".to_string(), "repo".to_string(), "kode".to_string()],
        },
        // CRM & Customer Support
        ToolDefinition {
            name: "zendesk_create_ticket".to_string(),
            server: "crm_support".to_string(),
            description: "Open new customer support ticket with priority and tags".to_string(),
            input_schema: json!({ "type": "object", "properties": { "subject": { "type": "string" }, "body": { "type": "string" } }, "required": ["subject"] }),
            required_params: vec!["subject".to_string()],
            when_to_use: "When customer support issues or inquiries are reported".to_string(),
            tags: vec!["ticket".to_string(), "support".to_string(), "bantuan".to_string(), "keluhan".to_string()],
        },
        ToolDefinition {
            name: "redis_cache_invalidate".to_string(),
            server: "cache_kv".to_string(),
            description: "Purge key or namespace pattern from in-memory cluster".to_string(),
            input_schema: json!({ "type": "object", "properties": { "pattern": { "type": "string" } }, "required": ["pattern"] }),
            required_params: vec!["pattern".to_string()],
            when_to_use: "When invalidating cached session keys or flushing memory".to_string(),
            tags: vec!["redis".to_string(), "cache".to_string(), "purge".to_string(), "hapus".to_string(), "flush".to_string()],
        },
        ToolDefinition {
            name: "transcode_media_video".to_string(),
            server: "mediavault".to_string(),
            description: "Transcode raw video stream into optimized HLS and MP4".to_string(),
            input_schema: json!({ "type": "object", "properties": { "video_id": { "type": "string" } }, "required": ["video_id"] }),
            required_params: vec!["video_id".to_string()],
            when_to_use: "When encoding video files into streaming formats".to_string(),
            tags: vec!["video".to_string(), "transcode".to_string(), "media".to_string(), "hls".to_string()],
        },
    ];

    // Seed up to 50 tools to test high-cardinality indexing out-of-the-box
    for i in 19..=50 {
        let cat = match i % 4 {
            0 => "database",
            1 => "storage",
            2 => "notification",
            _ => "billing",
        };
        tools.push(ToolDefinition {
            name: format!("{}_worker_task_{}", cat, i),
            server: format!("{}_cluster", cat),
            description: format!("Automated enterprise background processor on {} node #{}", cat, i),
            input_schema: json!({ "type": "object" }),
            required_params: vec![],
            when_to_use: format!("When dispatching asynchronous operations on {} cluster", cat),
            tags: vec![cat.to_string(), "worker".to_string(), "task".to_string(), format!("node_{}", i)],
        });
    }

    tools
}
