<script lang="ts">
  import { Activity, Shield, Users, Server, ArrowUpRight, CheckCircle2 } from 'lucide-svelte';

  const metrics = [
    { label: 'Requests / Sec', value: '1,420 rps', change: '+12.4%', icon: Activity },
    { label: 'P99 Latency', value: '0.38 ms', change: '-4.1%', icon: CheckCircle2 },
    { label: 'Active Agent Sessions', value: '48 nodes', change: '8 clusters', icon: Users },
    { label: 'Security Blocks (DLP/WAF)', value: '14 incidents', change: '100% neutralized', icon: Shield },
  ];

  const recentCalls = [
    { time: '18:42:01.104', agent: 'agent.claude.prod-01', tool: 'duckdb_query_vault', duration: '0.24ms', status: 'SUCCESS' },
    { time: '18:42:00.892', agent: 'agent.cursor.dev-04', tool: 'fetch_git_commit', duration: '0.41ms', status: 'SUCCESS' },
    { time: '18:41:59.710', agent: 'agent.antigravity.node', tool: 'execute_sql_sandbox', duration: '1.12ms', status: 'MODIFIED_DLP' },
    { time: '18:41:58.204', agent: 'agent.external.bot', tool: 'transfer_funds', duration: '0.18ms', status: 'SUSPENDED_HITL' },
  ];
</script>

<div class="space-y-4 sm:space-y-6">
  <!-- Top Metric Cards -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4">
    {#each metrics as m}
      <div class="rounded-lg border border-border bg-card p-4 transition-colors hover:border-muted-foreground/30 shadow-xs">
        <div class="flex items-center justify-between text-muted-foreground text-xs font-medium">
          <span>{m.label}</span>
          <svelte:component this={m.icon} class="h-4 w-4 shrink-0 text-muted-foreground/80" />
        </div>
        <div class="mt-2 text-xl sm:text-2xl font-bold tracking-tight text-foreground tabular-nums">
          {m.value}
        </div>
        <div class="mt-1 flex items-center text-xs text-muted-foreground">
          <span class="text-emerald-500 font-medium tabular-nums">{m.change}</span>
        </div>
      </div>
    {/each}
  </div>

  <!-- Realtime Invocations Table -->
  <div class="rounded-lg border border-border bg-card overflow-hidden shadow-xs">
    <div class="p-4 border-b border-border flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <div>
        <h3 class="text-sm font-semibold tracking-tight">Recent Wire Invocations</h3>
        <p class="text-xs text-muted-foreground">Request traces evaluated via stateless headers and credential proxy.</p>
      </div>
      <div class="flex items-center gap-2 self-start sm:self-auto">
        <span class="inline-flex items-center rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-500 tabular-nums">
          ALPN h2 / TLS 1.3
        </span>
      </div>
    </div>
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
  </div>
</div>
