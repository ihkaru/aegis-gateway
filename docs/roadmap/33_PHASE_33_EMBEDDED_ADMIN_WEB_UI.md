# Phase 33: Single-Binary Embedded Admin Web UI (Svelte 5, Bun, shadcn-svelte, Multi-Theme)

## Executive Summary
Phase 33 provides a zero-dependency, single-binary Administrative Web Dashboard for Aegis Gateway. Built with Svelte 5 (Runes), Bun, TypeScript, and shadcn-svelte, the interface delivers high information density and instant observability for Security Officers, CISOs, and Platform Architects. It enforces strict "anti-slop" engineering principles: clean neo-grotesque typography, tabular numbers, monochrome geometric SVG icons (Lucide), multiple enterprise themes (Zinc, Slate, Neutral, OLED Black), and a strict zero-emoji policy.

---

## 1. Architectural Invariants & Requirements

1. **Modern Frontend Stack & Tooling**:
   - Svelte 5 (Runes `$state`, `$derived`, `$props`) with TypeScript.
   - Built and bundled exclusively via **Bun** (`bun run build`).
   - Component architecture adhering to `shadcn-svelte` and Tailwind CSS variables.
2. **Multi-Theme & Anti-Slop Aesthetics**:
   - Palette switching: Zinc, Slate, Neutral, and OLED Black (`#000000`).
   - Typography: Clean sans-serif with monospace tabular figures (`tnum`) for realtime metrics and latencies.
   - Iconography: Monochrome 1.5px/2px SVG strokes from `lucide-svelte`.
   - **Zero Emoji Policy**: Absolutely no emoticons/emojis in buttons, badges, tables, or navigation.
3. **Single-Binary Embedding & SPA Fallback**:
   - Static assets bundled into the Rust binary via compile-time asset embedding (`rust-embed`).
   - Axum router handles SPA fallback: API routes (`/api/v1/*`) are processed by handlers, while client navigation routes (`/dashboard`, `/dlp`, `/hitl`, `/finops`) serve `index.html`.
4. **Port & Control Plane Isolation**:
   - Management UI listens on a dedicated administration port (`:8485`), completely isolated from the high-throughput MCP data plane (`:8484`).
   - Fully toggleable at runtime via `--enable-ui` / `ui.enabled = false` for headless Kubernetes deployments.

---

## 2. Verification Criteria

- [x] Production build of the Svelte 5 application bundles successfully via Bun into `ui/dist/`.
- [x] Embedded Axum router correctly serves static assets with proper MIME types.
- [x] Client-side routing fallback serves `index.html` on non-API paths.
- [x] Admin REST endpoints expose live system metrics, DLP audit events, HITL tasks, and FinOps quotas.
