<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import { Activity, Server, Shield, Terminal, Zap, ArrowRight, AlertTriangle } from 'lucide-svelte';

  let { data } = $props<{
    data: {
      label: string;
      node_type: string;
      status: string;
      metrics?: {
        rps: number;
        p99_latency_ms: number;
        active_connections: number;
        error_rate: number;
      };
      metadata?: Record<string, string>;
    };
  }>();

  const isHealthy = $derived(data.status === 'healthy');
  const isTripped = $derived(data.status === 'circuit_open' || data.status === 'degraded');
</script>

<div class="relative rounded-lg border bg-card p-3 shadow-sm min-w-[210px] max-w-[250px] transition-all duration-200 {isTripped ? 'border-destructive ring-1 ring-destructive/40' : 'border-border hover:border-primary/50'}">
  <!-- Incoming Handle -->
  <Handle
    type="target"
    position={Position.Left}
    class="!bg-primary !w-2 !h-2 !border-2 !border-background"
  />

  <!-- Header: Category Badge & Status Indicator -->
  <div class="flex items-center justify-between gap-2 mb-2">
    <span class="text-[10px] font-semibold tracking-wider uppercase px-1.5 py-0.5 rounded border {
      data.node_type === 'client' ? 'bg-sky-500/10 text-sky-500 border-sky-500/30' :
      data.node_type === 'ingress' ? 'bg-blue-500/10 text-blue-500 border-blue-500/30' :
      data.node_type === 'filter' ? 'bg-purple-500/10 text-purple-500 border-purple-500/30' :
      data.node_type === 'router' ? 'bg-cyan-500/10 text-cyan-500 border-cyan-500/30' :
      data.node_type === 'backend' ? 'bg-emerald-500/10 text-emerald-500 border-emerald-500/30' :
      'bg-amber-500/10 text-amber-500 border-amber-500/30'
    }">
      {data.node_type}
    </span>

    <div class="flex items-center gap-1.5 text-[11px] font-medium">
      {#if isHealthy}
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
        </span>
        <span class="text-emerald-500 font-mono text-[10px]">OK</span>
      {:else}
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-rose-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-rose-500"></span>
        </span>
        <span class="text-rose-500 font-mono text-[10px]">TRIPPED</span>
      {/if}
    </div>
  </div>

  <!-- Node Title -->
  <div class="font-semibold text-xs text-foreground tracking-tight line-clamp-1 mb-1.5">
    {data.label}
  </div>

  <!-- Metrics Bar -->
  {#if data.metrics}
    <div class="flex items-center justify-between text-[10px] text-muted-foreground pt-1.5 border-t border-border font-mono">
      <span>{data.metrics.rps.toFixed(1)} RPS</span>
      <span>{data.metrics.p99_latency_ms.toFixed(2)}ms</span>
    </div>
  {/if}

  <!-- Outgoing Handle -->
  <Handle
    type="source"
    position={Position.Right}
    class="!bg-primary !w-2 !h-2 !border-2 !border-background"
  />
</div>
