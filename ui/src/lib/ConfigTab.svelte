<script lang="ts">
  import { onMount } from 'svelte';
  import {
    FileText,
    CheckCircle2,
    AlertTriangle,
    RotateCcw,
    Save,
    Server,
    ShieldCheck,
    RefreshCw,
  } from 'lucide-svelte';

  interface ConfigState {
    path: string;
    format: string;
    content: string;
    status: string;
  }

  interface ValidationResult {
    valid: boolean;
    backends_count?: number;
    default_tenant?: string;
    backends?: string[];
    error?: string;
  }

  interface ApplyResult {
    status: string;
    message?: string;
    backends_count?: number;
    reloaded_backends?: string[];
    default_tenant?: string;
    error?: string;
  }

  let configText = $state('');
  let configPath = $state('propylon.yaml');
  let configFormat = $state('yaml');
  let loading = $state(true);
  let validating = $state(false);
  let applying = $state(false);
  let validationResult = $state<ValidationResult | null>(null);
  let applyResult = $state<ApplyResult | null>(null);

  async function loadCurrentConfig() {
    loading = true;
    applyResult = null;
    try {
      const res = await fetch('/api/v1/config/raw');
      if (res.ok) {
        const data: ConfigState = await res.json();
        configText = data.content;
        configPath = data.path;
        configFormat = data.format;
        await validateSyntax(data.content);
      }
    } catch (e: any) {
      applyResult = { status: 'ERROR', error: e.message };
    } finally {
      loading = false;
    }
  }

  async function validateSyntax(contentToValidate?: string) {
    validating = true;
    const content = contentToValidate ?? configText;
    try {
      const res = await fetch('/api/v1/config/validate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content, format: configFormat }),
      });
      validationResult = await res.json();
    } catch (e: any) {
      validationResult = { valid: false, error: e.message };
    } finally {
      validating = false;
    }
  }

  async function applyAndHotReload() {
    applying = true;
    applyResult = null;
    try {
      const res = await fetch('/api/v1/config/apply', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: configText, format: configFormat }),
      });
      applyResult = await res.json();
      if (res.ok && applyResult?.status === 'APPLIED') {
        await validateSyntax(configText);
      }
    } catch (e: any) {
      applyResult = { status: 'ERROR', error: e.message };
    } finally {
      applying = false;
    }
  }

  async function resetToDefaultTemplate() {
    try {
      const res = await fetch('/api/v1/config/template');
      if (res.ok) {
        const data = await res.json();
        configText = data.content;
        await validateSyntax(data.content);
      }
    } catch {
      // Ignore
    }
  }

  onMount(() => {
    loadCurrentConfig();
  });
</script>

<div class="space-y-4 font-sans text-xs">
  <!-- Header Banner -->
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight flex items-center gap-2">
        <FileText class="h-4 w-4 text-primary" />
        Topology & Runtime Configuration Editor
      </h3>
      <p class="text-xs text-muted-foreground">
        Live interactive declarative YAML/JSON configuration with atomic zero-downtime hot-reload and preflight validation.
      </p>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={resetToDefaultTemplate}
        class="flex items-center gap-1.5 rounded border border-border bg-muted/40 px-2.5 py-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Load Standard Enterprise Starter Template"
      >
        <RotateCcw class="h-3 w-3" />
        Starter Template
      </button>
      <button
        onclick={loadCurrentConfig}
        class="flex items-center gap-1.5 rounded border border-border bg-muted/40 px-2.5 py-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Reload saved file from disk"
      >
        <RefreshCw class="h-3 w-3 {loading ? 'animate-spin' : ''}" />
        Reload
      </button>
    </div>
  </div>

  <!-- Validation Status Summary Banner -->
  <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
    <div class="rounded-md border border-border bg-card p-3 flex items-center justify-between">
      <div class="space-y-0.5">
        <span class="text-[10px] uppercase font-bold tracking-wider text-muted-foreground">File Target</span>
        <div class="font-mono text-xs font-semibold text-foreground truncate">{configPath}</div>
      </div>
      <span class="rounded bg-primary/10 border border-primary/20 px-1.5 py-0.5 font-mono text-[10px] text-primary font-bold uppercase">
        {configFormat}
      </span>
    </div>

    <div class="rounded-md border border-border bg-card p-3 flex items-center justify-between">
      <div class="space-y-0.5">
        <span class="text-[10px] uppercase font-bold tracking-wider text-muted-foreground">Validation Status</span>
        <div class="flex items-center gap-1.5">
          {#if validating}
            <span class="text-xs text-muted-foreground">Validating...</span>
          {:else if validationResult?.valid}
            <CheckCircle2 class="h-3.5 w-3.5 text-emerald-500" />
            <span class="text-xs font-semibold text-emerald-500">VALID SCHEMA</span>
          {:else if validationResult?.error}
            <AlertTriangle class="h-3.5 w-3.5 text-rose-500" />
            <span class="text-xs font-semibold text-rose-500">SYNTAX ERROR</span>
          {:else}
            <span class="text-xs text-muted-foreground">Unchecked</span>
          {/if}
        </div>
      </div>
      <span class="font-mono text-xs text-muted-foreground">
        {validationResult?.backends_count ?? 0} backends
      </span>
    </div>

    <div class="rounded-md border border-border bg-card p-3 flex items-center justify-between">
      <div class="space-y-0.5">
        <span class="text-[10px] uppercase font-bold tracking-wider text-muted-foreground">Default Tenant</span>
        <div class="font-mono text-xs font-semibold text-foreground truncate">
          {validationResult?.default_tenant || 'enterprise-corp'}
        </div>
      </div>
      <ShieldCheck class="h-4 w-4 text-emerald-500" />
    </div>
  </div>

  <!-- Notification Banner for Apply/Error -->
  {#if applyResult}
    {#if applyResult.status === 'APPLIED'}
      <div class="rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 flex items-start gap-2.5 text-emerald-500">
        <CheckCircle2 class="h-4 w-4 shrink-0 mt-0.5" />
        <div class="space-y-0.5">
          <div class="font-semibold text-xs">{applyResult.message}</div>
          <div class="text-[11px] text-emerald-500/80 font-mono">
            Active Backends: {applyResult.backends_count} (Reloaded: {applyResult.reloaded_backends?.join(', ') || 'in-sync'})
          </div>
        </div>
      </div>
    {:else}
      <div class="rounded-md border border-rose-500/30 bg-rose-500/10 p-3 flex items-start gap-2.5 text-rose-500">
        <AlertTriangle class="h-4 w-4 shrink-0 mt-0.5" />
        <div class="space-y-0.5">
          <div class="font-semibold text-xs">Configuration Apply Failed</div>
          <div class="text-[11px] text-rose-500/80 font-mono break-all">{applyResult.error}</div>
        </div>
      </div>
    {/if}
  {/if}

  <!-- Error feedback if validation failed -->
  {#if validationResult?.error}
    <div class="rounded-md border border-rose-500/30 bg-rose-500/10 p-2.5 flex items-start gap-2 text-rose-500">
      <AlertTriangle class="h-3.5 w-3.5 shrink-0 mt-0.5" />
      <div class="text-[11px] font-mono break-all">{validationResult.error}</div>
    </div>
  {/if}

  <!-- Editor Container -->
  <div class="rounded-lg border border-border bg-card overflow-hidden shadow-xs">
    <div class="border-b border-border bg-muted/30 px-3 py-2 flex items-center justify-between">
      <div class="flex items-center gap-2 text-xs font-medium text-muted-foreground">
        <Server class="h-3.5 w-3.5" />
        <span>Declarative Topology Specification</span>
      </div>
      <div class="flex items-center gap-2">
        <button
          onclick={() => validateSyntax()}
          disabled={validating}
          class="flex items-center gap-1 rounded border border-border bg-card px-2 py-1 text-[11px] font-medium text-foreground hover:bg-muted transition-colors"
        >
          <ShieldCheck class="h-3 w-3 text-primary" />
          Preflight Check
        </button>
        <button
          onclick={applyAndHotReload}
          disabled={applying || (validationResult != null && !validationResult.valid)}
          class="flex items-center gap-1 rounded bg-primary text-primary-foreground px-3 py-1 text-[11px] font-semibold hover:bg-primary/90 transition-colors shadow-xs disabled:opacity-50"
        >
          <Save class="h-3 w-3" />
          {applying ? 'Applying...' : 'Apply & Hot-Reload'}
        </button>
      </div>
    </div>

    <div class="p-3 bg-muted/10">
      <textarea
        bind:value={configText}
        oninput={() => validateSyntax()}
        rows="16"
        spellcheck="false"
        class="w-full rounded border border-border bg-background p-3 font-mono text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary leading-relaxed resize-y"
        placeholder="mcp_servers:&#10;  sqlite_ledger:&#10;    name: sqlite_ledger&#10;    command: python3&#10;    args:&#10;    - scripts/sqlite_mcp_server.py&#10;    timeout_secs: 30&#10;    enabled: true&#10;default_tenant: enterprise-corp"
      ></textarea>
    </div>
    <div class="border-t border-border bg-muted/20 px-3 py-2 text-[10px] text-muted-foreground flex items-center justify-between">
      <span>Zero-downtime hot-reload swaps memory state without disrupting streaming HTTP/SSE sessions.</span>
      <span>Lines: {configText.split('\n').length} | Chars: {configText.length}</span>
    </div>
  </div>
</div>
