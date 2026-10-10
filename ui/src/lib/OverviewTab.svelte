<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, Shield, Users, Server, CheckCircle2, Cpu, RefreshCw } from 'lucide-svelte';

  interface OverviewData {
    status: string;
    uptime_secs: number;
    rps: number;
    p99_latency_ms: number;
    active_agents: number;
    active_backends: number;
    total_tools: number;
    memory_rss_mb: number;
    mtls_enforced: boolean;
    policy_tier: string;
  }

  interface WireCall {
    time: string;
    agent: string;
    tool: string;
    duration: string;
    status: string;
  }

  let overview = $state<OverviewData>({
    status: 'ONLINE',
    uptime_secs: 0,
    rps: 0,
    p99_latency_ms: 0.38,
    active_agents: 0,
    active_backends: 0,
    total_tools: 0,
    memory_rss_mb: 0.0,
    mtls_enforced: true,
    policy_tier: 'Hybrid',
  });

  let recentCalls = $state<WireCall[]>([]);
  let loading = $state(true);

  async function fetchLiveMetrics() {
    try {
      const res = await fetch('/api/v1/overview');
      if (res.ok) {
        overview = await res.json();
      }
      const callsRes = await fetch('/api/v1/recent_calls');
      if (callsRes.ok) {
        recentCalls = await callsRes.json();
      }
    } catch {
      // Retain state during transient errors
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchLiveMetrics();
    const timer = setInterval(fetchLiveMetrics, 3000);
    return () => clearInterval(timer);
  });
</script>

<div class="space-y-4 sm:space-y-6 font-sans">
  <!-- Top Metric Cards with Live Data Plane Engine Telemetry -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4">
    <div class="rounded-lg border border-border bg-card p-4 transition-colors hover:border-muted-foreground/30 shadow-xs">
      <div class="flex items-center justify-between text-muted-foreground text-xs font-medium">
        <span>Memory RSS Usage</span>
        <Cpu class="h-4 w-4 shrink-0 text-muted-foreground/80" />
      </div>
      <div class="mt-2 text-xl sm:text-2xl font-bold tracking-tight text-foreground tabular-nums">
        {overview.memory_rss_mb.toFixed(1)} MB
      </div>
      <div class="mt-1 flex items-center text-xs text-muted-foreground">
        <span class="text-emerald-500 font-medium tabular-nums">Linux /proc RSS</span>
      </div>
    </div>

    <div class="rounded-lg border border-border bg-card p-4 transition-colors hover:border-muted-foreground/30 shadow-xs">
      <div class="flex items-center justify-between text-muted-foreground text-xs font-medium">
        <span>P99 Wire Latency</span>
        <CheckCircle2 class="h-4 w-4 shrink-0 text-muted-foreground/80" />
      </div>
      <div class="mt-2 text-xl sm:text-2xl font-bold tracking-tight text-foreground tabular-nums">
        {overview.p99_latency_ms.toFixed(2)} ms
      </div>
      <div class="mt-1 flex items-center text-xs text-muted-foreground">
        <span class="text-emerald-500 font-medium tabular-nums">Zero-Copy Stateless</span>
      </div>
    </div>

    <div class="rounded-lg border border-border bg-card p-4 transition-colors hover:border-muted-foreground/30 shadow-xs">
      <div class="flex items-center justify-between text-muted-foreground text-xs font-medium">
        <span>Registered Backends</span>
        <Server class="h-4 w-4 shrink-0 text-muted-foreground/80" />
      </div>
      <div class="mt-2 text-xl sm:text-2xl font-bold tracking-tight text-foreground tabular-nums">
        {overview.active_backends} Nodes
      </div>
      <div class="mt-1 flex items-center text-xs text-muted-foreground">
        <span class="text-primary font-medium tabular-nums">{overview.total_tools} tools registered</span>
      </div>
    </div>

    <div class="rounded-lg border border-border bg-card p-4 transition-colors hover:border-muted-foreground/30 shadow-xs">
      <div class="flex items-center justify-between text-muted-foreground text-xs font-medium">
        <span>Policy Tier Status</span>
        <Shield class="h-4 w-4 shrink-0 text-muted-foreground/80" />
      </div>
      <div class="mt-2 text-xl sm:text-2xl font-bold tracking-tight text-foreground tabular-nums">
        {overview.policy_tier}
      </div>
      <div class="mt-1 flex items-center text-xs text-muted-foreground">
        <span class="text-emerald-500 font-medium tabular-nums">mTLS & DLP Active</span>
      </div>
    </div>
  </div>

  <!-- Realtime Invocations Table -->
  <div class="rounded-lg border border-border bg-card overflow-hidden shadow-xs">
    <div class="p-4 border-b border-border flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <div>
        <h3 class="text-sm font-semibold tracking-tight">Recent Wire Invocations</h3>
        <p class="text-xs text-muted-foreground">Live request traces evaluated via stateless headers and credential proxy.</p>
      </div>
      <div class="flex items-center gap-2 self-start sm:self-auto">
        <span class="inline-flex items-center rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-500 tabular-nums">
          ALPN h2 / TLS 1.3
        </span>
      </div>
    </div>

    {#if recentCalls.length === 0}
      <div class="p-8 text-center text-xs text-muted-foreground font-sans border-t border-border">
        No wire invocations recorded in current buffer. Gateway is active and listening for agent traffic.
      </div>
    {:else}
      <div class="overflow-x-auto w-full">
        <table class="w-full text-left text-xs min-w-[520px]">
          <thead class="bg-muted/40 text-muted-foreground uppercase text-[10px] tracking-wider border-b border-border">
            <tr>
              <th class="p-3 whitespace-nowrap">Timestamp</th>
              <th class="p-3">Agent Identity</th>
              <th class="p-3">Tool Invocated</th>
              <th class="p-3 whitespace-nowrap">Latency</th>
              <th class="p-3">Status</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each recentCalls as call}
              <tr class="hover:bg-muted/30 transition-colors">
                <td class="p-3 text-muted-foreground tabular-nums whitespace-nowrap">{call.time}</td>
                <td class="p-3 font-semibold text-foreground break-all">{call.agent}</td>
                <td class="p-3 text-primary font-medium break-all">{call.tool}</td>
                <td class="p-3 text-muted-foreground tabular-nums whitespace-nowrap">{call.duration}</td>
                <td class="p-3 whitespace-nowrap">
                  {#if call.status === 'SUCCESS'}
                    <span class="inline-flex items-center rounded border border-emerald-500/20 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-500">
                      OK
                    </span>
                  {:else if call.status === 'MODIFIED_DLP'}
                    <span class="inline-flex items-center rounded border border-amber-500/20 bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-medium text-amber-500">
                      DLP_MASKED
                    </span>
                  {:else}
                    <span class="inline-flex items-center rounded border border-rose-500/20 bg-rose-500/10 px-1.5 py-0.5 text-[10px] font-medium text-rose-500">
                      HITL_HELD
                    </span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>
