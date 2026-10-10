<script lang="ts">
  import { Coins, AlertOctagon, TrendingUp, CheckCircle, Ban } from 'lucide-svelte';

  const tenants = [
    {
      name: 'Tenant: Algorithmic-Trading',
      id: 'tnt_algo_99',
      spentUsd: '$840.20',
      limitUsd: '$1,000.00',
      percentage: 84,
      status: 'SOFT_WARNING',
      tokensUsed: '12.4M',
    },
    {
      name: 'Tenant: Customer-Support-Bot',
      id: 'tnt_support_01',
      spentUsd: '$190.50',
      limitUsd: '$500.00',
      percentage: 38,
      status: 'NORMAL',
      tokensUsed: '4.1M',
    },
    {
      name: 'Tenant: Autonomous-Scraper',
      id: 'tnt_scrape_08',
      spentUsd: '$2,500.00',
      limitUsd: '$2,500.00',
      percentage: 100,
      status: 'HARD_FROZEN',
      tokensUsed: '45.0M',
    },
  ];
</script>

<div class="space-y-4">
  <div class="rounded-lg border border-border bg-card p-4">
    <h3 class="text-sm font-semibold tracking-tight">FinOps Multi-Tenant Quotas & Cost Governance</h3>
    <p class="text-xs text-muted-foreground">Real-time token metering with automatic hard freeze cutoff preventing runaway agent budget depletion.</p>
  </div>

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
</div>
