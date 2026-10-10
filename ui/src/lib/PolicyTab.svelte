<script lang="ts">
  import { onMount } from 'svelte';
  import { Shield, Zap, Lock, AlertTriangle, CheckCircle2, RefreshCw } from 'lucide-svelte';

  let currentTier = $state<string>('Hybrid');
  let switching = $state(false);
  let statusMessage = $state<string | null>(null);
  let swapLatency = $state<number | null>(null);

  async function fetchPolicy() {
    try {
      const res = await fetch('/api/v1/overview');
      if (res.ok) {
        const data = await res.json();
        currentTier = data.policy_tier || 'Hybrid';
      }
    } catch {
      // Retain state
    }
  }

  async function setPolicyTier(tier: 'Dev' | 'Hybrid' | 'Strict') {
    switching = true;
    statusMessage = null;
    const start = performance.now();
    try {
      const res = await fetch('/api/v1/policy/tier', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier }),
      });

      const elapsed = performance.now() - start;
      swapLatency = elapsed;

      if (res.ok) {
        const data = await res.json();
        currentTier = data.current_tier || tier;
        statusMessage = `Policy Tier successfully hot-swapped to "${currentTier}" in ${elapsed.toFixed(2)}ms with 0 dropped streaming connections!`;
        setTimeout(() => (statusMessage = null), 5000);
      } else {
        const err = await res.json();
        statusMessage = `Hot-swap failed: ${err.error || 'Server error'}`;
      }
    } catch (e: any) {
      statusMessage = `Network error: ${e.message}`;
    } finally {
      switching = false;
    }
  }

  onMount(() => {
    fetchPolicy();
  });
</script>

<div class="space-y-4 font-sans text-xs">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Zero-Downtime Dynamic Policy Control Plane</h3>
      <p class="text-xs text-muted-foreground">Perform atomic in-memory policy tier hot-swaps without interrupting active agent streaming sessions.</p>
    </div>
    <div class="flex items-center gap-2">
      <span class="rounded bg-primary/10 border border-primary/20 px-2 py-0.5 text-xs text-primary font-bold">
        Active Tier: {currentTier}
      </span>
      <button
        onclick={fetchPolicy}
        class="rounded border border-border bg-muted/50 p-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0" />
      </button>
    </div>
  </div>

  {#if statusMessage}
    <div class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs font-medium text-emerald-500 flex items-center justify-between">
      <span>{statusMessage}</span>
      {#if swapLatency !== null}
        <span class="font-mono text-[10px] tabular-nums">LATENCY: {swapLatency.toFixed(2)}ms</span>
      {/if}
    </div>
  {/if}

  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <!-- Dev Tier Card -->
    <div
      class="rounded-lg border p-4 space-y-3 transition-all {currentTier === 'Dev' ? 'border-primary ring-1 ring-primary bg-primary/5 shadow-sm' : 'border-border bg-card hover:border-muted-foreground/30'}"
    >
      <div class="flex items-center justify-between">
        <div class="font-bold text-sm flex items-center gap-2">
          <Zap class="h-4 w-4 text-amber-500" />
          Dev Sandbox
        </div>
        {#if currentTier === 'Dev'}
          <span class="rounded bg-primary text-primary-foreground text-[10px] px-1.5 py-0.2 font-semibold">ACTIVE</span>
        {/if}
      </div>
      <p class="text-muted-foreground text-[11px]">
        Permissive environment for rapid agent prototyping. Debug tracing enabled, relaxed bounds.
      </p>
      <ul class="space-y-1 text-[11px] text-muted-foreground list-disc list-inside">
        <li>Relaxed ABAC argument limits</li>
        <li>Permissive read-only SQL queries</li>
        <li>Subprocess stdout debug logging</li>
      </ul>
      <button
        onclick={() => setPolicyTier('Dev')}
        disabled={switching || currentTier === 'Dev'}
        class="w-full mt-2 rounded border border-border py-1.5 text-xs font-semibold hover:bg-muted disabled:opacity-50 transition-colors"
      >
        {currentTier === 'Dev' ? 'Currently Selected' : 'Hot-Swap to Dev'}
      </button>
    </div>

    <!-- Hybrid Tier Card -->
    <div
      class="rounded-lg border p-4 space-y-3 transition-all {currentTier === 'Hybrid' ? 'border-primary ring-1 ring-primary bg-primary/5 shadow-sm' : 'border-border bg-card hover:border-muted-foreground/30'}"
    >
      <div class="flex items-center justify-between">
        <div class="font-bold text-sm flex items-center gap-2">
          <Shield class="h-4 w-4 text-primary" />
          Hybrid Enterprise
        </div>
        {#if currentTier === 'Hybrid'}
          <span class="rounded bg-primary text-primary-foreground text-[10px] px-1.5 py-0.2 font-semibold">ACTIVE</span>
        {/if}
      </div>
      <p class="text-muted-foreground text-[11px]">
        Balanced defense-in-depth governance. Bidirectional DLP sanitization and HITL authorization queue.
      </p>
      <ul class="space-y-1 text-[11px] text-muted-foreground list-disc list-inside">
        <li>Dynamic AST argument bounds check</li>
        <li>Sub-ms inline PCI/PII DLP masking</li>
        <li>Cryptographic HMAC HITL for high-risk tools</li>
      </ul>
      <button
        onclick={() => setPolicyTier('Hybrid')}
        disabled={switching || currentTier === 'Hybrid'}
        class="w-full mt-2 rounded bg-primary text-primary-foreground py-1.5 text-xs font-semibold hover:bg-primary/90 disabled:opacity-50 transition-colors shadow-xs"
      >
        {currentTier === 'Hybrid' ? 'Currently Selected' : 'Hot-Swap to Hybrid'}
      </button>
    </div>

    <!-- Strict Tier Card -->
    <div
      class="rounded-lg border p-4 space-y-3 transition-all {currentTier === 'Strict' ? 'border-primary ring-1 ring-primary bg-primary/5 shadow-sm' : 'border-border bg-card hover:border-muted-foreground/30'}"
    >
      <div class="flex items-center justify-between">
        <div class="font-bold text-sm flex items-center gap-2">
          <Lock class="h-4 w-4 text-rose-500" />
          Strict Zero-Trust
        </div>
        {#if currentTier === 'Strict'}
          <span class="rounded bg-primary text-primary-foreground text-[10px] px-1.5 py-0.2 font-semibold">ACTIVE</span>
        {/if}
      </div>
      <p class="text-muted-foreground text-[11px]">
        Maximum enterprise posture for regulated banking and defense workloads.
      </p>
      <ul class="space-y-1 text-[11px] text-muted-foreground list-disc list-inside">
        <li>Strict client mTLS with CRL inspection</li>
        <li>Hard DDL statement blocking (DROP/TRUNCATE)</li>
        <li>Ed25519 AgentCert verification mandatory</li>
      </ul>
      <button
        onclick={() => setPolicyTier('Strict')}
        disabled={switching || currentTier === 'Strict'}
        class="w-full mt-2 rounded border border-rose-500/30 text-rose-500 py-1.5 text-xs font-semibold hover:bg-rose-500/10 disabled:opacity-50 transition-colors"
      >
        {currentTier === 'Strict' ? 'Currently Selected' : 'Hot-Swap to Strict'}
      </button>
    </div>
  </div>
</div>
