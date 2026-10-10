<script lang="ts">
  import { onMount } from 'svelte';
  import { Coins, AlertOctagon, TrendingUp, CheckCircle, Ban, RefreshCw } from 'lucide-svelte';

  interface TenantFinops {
    name: string;
    id: string;
    spentUsd: string;
    limitUsd: string;
    percentage: number;
    status: 'NORMAL' | 'SOFT_WARNING' | 'HARD_FROZEN';
    tokensUsed: string;
  }

  let tenants = $state<TenantFinops[]>([]);
  let totalMetered = $state('0');
  let loading = $state(true);

  async function fetchFinops() {
    try {
      const res = await fetch('/api/v1/finops');
      if (res.ok) {
        const data = await res.json();
        tenants = data.tenants || [];
        totalMetered = data.total_tokens_metered || '0';
      }
    } catch {
      // Retain state
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchFinops();
    const interval = setInterval(fetchFinops, 3000);
    return () => clearInterval(interval);
  });
</script>

<div class="space-y-4 font-sans">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">FinOps Multi-Tenant Quotas & Cost Governance</h3>
      <p class="text-xs text-muted-foreground">Real-time token metering with automatic hard freeze cutoff preventing runaway agent budget depletion.</p>
    </div>
    <div class="flex items-center gap-2">
      <span class="rounded border border-primary/20 bg-primary/10 px-2 py-1 text-xs text-primary font-semibold tabular-nums">
        Metered: {totalMetered} Tokens
      </span>
      <button
        onclick={fetchFinops}
        class="rounded border border-border bg-muted/50 p-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>

  {#if tenants.length === 0}
    <div class="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground font-sans">
      No multi-tenant budget records active in current window. FinOps hard freeze engine is armed and monitoring.
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-3 sm:gap-4 font-sans">
      {#each tenants as t}
        <div class="rounded-lg border border-border bg-card p-4 space-y-3 shadow-xs">
          <div class="flex items-center justify-between gap-2">
            <span class="text-xs font-bold text-foreground truncate">{t.name}</span>
            {#if t.status === 'HARD_FROZEN'}
              <span class="shrink-0 rounded bg-rose-500/10 border border-rose-500/20 px-1.5 py-0.5 text-[10px] font-bold text-rose-500 flex items-center gap-1">
                <Ban class="h-3 w-3" />
                FROZEN
              </span>
            {:else if t.status === 'SOFT_WARNING'}
              <span class="shrink-0 rounded bg-amber-500/10 border border-amber-500/20 px-1.5 py-0.5 text-[10px] font-bold text-amber-500">
                WARNING
              </span>
            {:else}
              <span class="shrink-0 rounded bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 text-[10px] font-bold text-emerald-500">
                ACTIVE
              </span>
            {/if}
          </div>

          <div class="space-y-1.5">
            <div class="flex justify-between text-xs text-muted-foreground">
              <span>Budget Consumption</span>
              <span class="font-bold text-foreground tabular-nums">{t.spentUsd} / {t.limitUsd}</span>
            </div>
            <div class="h-2 w-full rounded-full bg-muted overflow-hidden">
              <div
                class="h-full transition-all duration-300 {t.percentage >= 100 ? 'bg-rose-500' : t.percentage >= 80 ? 'bg-amber-500' : 'bg-primary'}"
                style="width: {t.percentage}%"
              ></div>
            </div>
          </div>

          <div class="pt-2 border-t border-border flex justify-between text-[11px] text-muted-foreground">
            <span>Metered Tokens:</span>
            <span class="font-semibold text-foreground tabular-nums">{t.tokensUsed}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
