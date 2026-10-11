<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Terminal,
    Play,
    Search,
    ShieldCheck,
    RefreshCw,
    Layers,
    Database,
    CheckCircle2,
    AlertTriangle,
  } from 'lucide-svelte';

  interface ToolItem {
    name: string;
    backend: string;
    purpose: string;
    signature: string;
    schema: any;
    tags: string[];
    is_live?: boolean;
  }

  interface ExecutionResult {
    outcome: string;
    tool: string;
    latency_ms: number;
    result: any;
    dlp_modified: boolean;
  }

  let tools = $state<ToolItem[]>([]);
  let loading = $state(true);
  let searchQuery = $state('');
  let filterScope = $state<'all' | 'live' | 'templates'>('all');
  let disclosureLevel = $state<'L0' | 'L1' | 'L2'>('L0');
  let selectedTool = $state<ToolItem | null>(null);
  let inputArgs = $state('{\n  "query": "SELECT * FROM customer_accounts LIMIT 5"\n}');
  let executing = $state(false);
  let execResult = $state<ExecutionResult | null>(null);

  async function fetchTools() {
    try {
      const res = await fetch('/api/v1/tools');
      if (res.ok) {
        tools = await res.json();
        if (tools.length > 0 && !selectedTool) {
          selectTool(tools[0]);
        }
      }
    } catch {
      // Retain state
    } finally {
      loading = false;
    }
  }

  function isLiveTool(tool: ToolItem): boolean {
    return tool.is_live === true || tool.backend === 'propylon_local_engine' || tool.backend === 'sqlite_ledger';
  }

  function selectTool(tool: ToolItem) {
    selectedTool = tool;
    execResult = null;
    if (tool.name.includes('sqlite') || tool.name.includes('sql') || tool.name.includes('ledger')) {
      inputArgs = '{\n  "query": "SELECT * FROM customer_accounts LIMIT 5"\n}';
    } else if (tool.name === 'propylon_ping') {
      inputArgs = '{\n  "echo": "Propylon loopback roundtrip ping"\n}';
    } else if (tool.name.includes('search') || tool.name.includes('lookup')) {
      inputArgs = '{\n  "query": "NIST AI RMF compliance standard",\n  "limit": 5\n}';
    } else {
      inputArgs = JSON.stringify(tool.schema?.properties ? { ...tool.schema.properties } : { param: 'sample_value' }, null, 2);
    }
  }

  async function runToolExecution() {
    if (!selectedTool) return;
    executing = true;
    execResult = null;
    try {
      let parsed = {};
      try {
        parsed = JSON.parse(inputArgs);
      } catch {
        alert('Invalid JSON in arguments input');
        executing = false;
        return;
      }

      const res = await fetch('/api/v1/tools/call', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tool: selectedTool.name, arguments: parsed }),
      });

      if (res.ok) {
        execResult = await res.json();
      } else {
        const err = await res.json();
        execResult = { outcome: 'ERROR', tool: selectedTool.name, latency_ms: 0, result: { error: err.error || 'Execution failed' }, dlp_modified: false };
      }
    } catch (e: any) {
      execResult = { outcome: 'NETWORK_ERROR', tool: selectedTool.name, latency_ms: 0, result: { error: e.message }, dlp_modified: false };
    } finally {
      executing = false;
    }
  }

  let filteredTools = $derived(
    tools
      .filter((t) => {
        if (filterScope === 'live') return isLiveTool(t);
        if (filterScope === 'templates') return !isLiveTool(t);
        return true;
      })
      .filter(
        (t) =>
          t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          t.backend.toLowerCase().includes(searchQuery.toLowerCase()) ||
          t.purpose.toLowerCase().includes(searchQuery.toLowerCase())
      )
  );

  onMount(() => {
    fetchTools();
  });
</script>

<div class="space-y-4 font-sans text-xs">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Interactive Tools Catalog & Execution Playground</h3>
      <p class="text-xs text-muted-foreground">Progressive disclosure token optimization with real local database execution and preflight guardrail verification.</p>
    </div>
    <div class="flex items-center gap-2">
      <div class="flex items-center rounded border border-border bg-muted/40 p-0.5 text-xs font-medium">
        <button
          onclick={() => (disclosureLevel = 'L0')}
          class="px-2 py-0.5 rounded text-[10px] transition-colors {disclosureLevel === 'L0' ? 'bg-primary text-primary-foreground font-semibold' : 'text-muted-foreground hover:text-foreground'}"
          title="L0 Purpose Only (~50 tokens)"
        >
          L0 Purpose
        </button>
        <button
          onclick={() => (disclosureLevel = 'L1')}
          class="px-2 py-0.5 rounded text-[10px] transition-colors {disclosureLevel === 'L1' ? 'bg-primary text-primary-foreground font-semibold' : 'text-muted-foreground hover:text-foreground'}"
          title="L1 Function Signature"
        >
          L1 Signature
        </button>
        <button
          onclick={() => (disclosureLevel = 'L2')}
          class="px-2 py-0.5 rounded text-[10px] transition-colors {disclosureLevel === 'L2' ? 'bg-primary text-primary-foreground font-semibold' : 'text-muted-foreground hover:text-foreground'}"
          title="L2 Full JSON Schema"
        >
          L2 Full Schema
        </button>
      </div>
      <button
        onclick={fetchTools}
        class="rounded border border-border bg-muted/50 p-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-12 gap-4">
    <!-- Left Tools List -->
    <div class="lg:col-span-5 rounded-lg border border-border bg-card p-3 space-y-3 shadow-xs">
      <div class="space-y-2">
        <div class="relative">
          <Search class="h-3.5 w-3.5 absolute left-2.5 top-2.5 text-muted-foreground" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Filter tools by name or backend..."
            class="w-full rounded border border-border bg-background pl-8 pr-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
        <div class="flex items-center gap-1.5 text-[10px]">
          <button
            onclick={() => (filterScope = 'all')}
            class="px-2 py-0.5 rounded border transition-colors {filterScope === 'all' ? 'border-primary bg-primary/10 text-primary font-semibold' : 'border-border text-muted-foreground hover:text-foreground'}"
          >All ({tools.length})</button>
          <button
            onclick={() => (filterScope = 'live')}
            class="px-2 py-0.5 rounded border transition-colors {filterScope === 'live' ? 'border-emerald-500 bg-emerald-500/10 text-emerald-500 font-semibold' : 'border-border text-muted-foreground hover:text-foreground'}"
          >Live Active ({tools.filter(isLiveTool).length})</button>
          <button
            onclick={() => (filterScope = 'templates')}
            class="px-2 py-0.5 rounded border transition-colors {filterScope === 'templates' ? 'border-primary bg-primary/10 text-primary font-semibold' : 'border-border text-muted-foreground hover:text-foreground'}"
          >Templates ({tools.filter((t) => !isLiveTool(t)).length})</button>
        </div>
      </div>

      {#if filteredTools.length === 0}
        <div class="p-6 text-center text-muted-foreground">
          No tools match the current filter.
        </div>
      {:else}
        <div class="space-y-1.5 max-h-[500px] overflow-y-auto pr-1">
          {#each filteredTools as t (t.name)}
            <button
              onclick={() => selectTool(t)}
              class="w-full text-left p-2.5 rounded border transition-colors {selectedTool?.name === t.name ? 'border-primary bg-primary/5 text-foreground' : 'border-border/60 hover:bg-muted/40 text-muted-foreground'}"
            >
              <div class="flex items-center justify-between gap-2">
                <span class="font-semibold text-foreground text-xs">{t.name}</span>
                <div class="flex items-center gap-1.5">
                  {#if isLiveTool(t)}
                    <span class="rounded bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 text-[9px] font-semibold text-emerald-500 flex items-center gap-1">
                      <CheckCircle2 class="h-2.5 w-2.5 shrink-0" />
                      LIVE
                    </span>
                  {:else}
                    <span class="rounded bg-muted px-1.5 py-0.5 text-[9px] font-medium text-muted-foreground uppercase flex items-center gap-1">
                      <Layers class="h-2.5 w-2.5 shrink-0" />
                      TEMPLATE
                    </span>
                  {/if}
                  <span class="rounded bg-muted px-1.5 py-0.2 text-[10px] text-muted-foreground uppercase">{t.backend}</span>
                </div>
              </div>
              <div class="mt-1 text-[11px] text-muted-foreground line-clamp-2">
                {#if disclosureLevel === 'L0'}
                  {t.purpose}
                {:else if disclosureLevel === 'L1'}
                  <code class="text-primary font-mono text-[10px]">{t.signature}</code>
                {:else}
                  <pre class="font-mono text-[9px] text-muted-foreground max-h-16 overflow-hidden">{JSON.stringify(t.schema, null, 2)}</pre>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Right Interactive Execution Playground -->
    <div class="lg:col-span-7 rounded-lg border border-border bg-card p-4 space-y-3 shadow-xs">
      {#if !selectedTool}
        <div class="p-12 text-center text-muted-foreground">
          Select a tool from the catalog to launch the execution playground.
        </div>
      {:else}
        <div class="flex items-center justify-between border-b border-border pb-2">
          <div>
            <div class="font-semibold text-foreground text-sm flex items-center gap-2">
              <Terminal class="h-4 w-4 text-primary" />
              <span>{selectedTool.name}</span>
            </div>
            <div class="text-[11px] text-muted-foreground mt-0.5">{selectedTool.purpose}</div>
          </div>
          <button
            onclick={runToolExecution}
            disabled={executing}
            class="inline-flex items-center gap-1.5 rounded bg-primary text-primary-foreground px-3 py-1.5 font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors shadow-xs"
          >
            <Play class="h-3.5 w-3.5 fill-current" />
            {executing ? 'Executing...' : 'Dispatch Tool'}
          </button>
        </div>

        {#if isLiveTool(selectedTool)}
          <div class="rounded border border-emerald-500/20 bg-emerald-500/5 p-2.5 text-xs flex items-start gap-2">
            <Database class="h-4 w-4 text-emerald-500 shrink-0 mt-0.5" />
            <div>
              <div class="font-medium text-foreground text-[11px]">Live Engine Attached</div>
              <div class="text-muted-foreground text-[10px] mt-0.5">Connected to active local backend. Queries and mutations execute on persistent storage and real process telemetry.</div>
            </div>
          </div>
        {:else}
          <div class="rounded border border-amber-500/20 bg-amber-500/5 p-2.5 text-xs flex items-start gap-2">
            <AlertTriangle class="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
            <div>
              <div class="font-medium text-foreground text-[11px]">Unattached Catalog Template</div>
              <div class="text-muted-foreground text-[10px] mt-0.5">Upstream backend not connected. Dispatching evaluates live ABAC and DLP guardrails in preflight mode without external side effects.</div>
            </div>
          </div>
        {/if}

        <div>
          <label for="arguments-editor" class="block font-medium text-muted-foreground mb-1">Invocation Arguments (JSON Payload):</label>
          <textarea
            id="arguments-editor"
            bind:value={inputArgs}
            rows="5"
            class="w-full rounded border border-border bg-background p-2 font-mono text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          ></textarea>
        </div>

        {#if execResult}
          <div class="rounded border border-border bg-muted/20 p-3 space-y-2">
            <div class="flex items-center justify-between">
              <span class="font-semibold text-foreground text-xs flex items-center gap-1.5">
                <ShieldCheck class="h-3.5 w-3.5 text-emerald-500" />
                Execution Response
              </span>
              <div class="flex items-center gap-2 text-[10px] tabular-nums">
                {#if execResult.result?.execution_mode === 'PREFLIGHT_GUARDRAIL_ONLY' || execResult.result?.data?.status === 'unattached'}
                  <span class="rounded bg-amber-500/10 border border-amber-500/20 px-1.5 py-0.5 text-amber-500 font-semibold flex items-center gap-1">
                    <ShieldCheck class="h-3 w-3" />
                    PREFLIGHT VERIFIED
                  </span>
                {:else}
                  <span class="rounded bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 text-emerald-500 font-semibold flex items-center gap-1">
                    <CheckCircle2 class="h-3 w-3" />
                    LIVE EXECUTED
                  </span>
                {/if}
                <span class="text-muted-foreground">{execResult.latency_ms.toFixed(2)} ms</span>
              </div>
            </div>

            <pre class="rounded bg-background border border-border p-2 font-mono text-[11px] text-foreground max-h-48 overflow-y-auto">{JSON.stringify(execResult.result, null, 2)}</pre>

            {#if execResult.dlp_modified}
              <div class="pt-2 border-t border-border/60 text-[10px] text-amber-500 font-medium flex items-center gap-1">
                <span>DLP Pipeline Active: Payload was inspected and sanitized before transmission.</span>
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>
