<script lang="ts">
  import { onMount } from 'svelte';
  import {
    SvelteFlow,
    Controls,
    Background,
    MiniMap,
    type Node,
    type Edge,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import {
    Activity,
    RefreshCw,
    X,
    Shield,
    Server,
    Zap,
    CheckCircle2,
    AlertTriangle,
    Sliders,
  } from 'lucide-svelte';
  import CustomGatewayNode from './CustomGatewayNode.svelte';

  const nodeTypes = {
    custom: CustomGatewayNode,
  };

  interface TopologyNodeData {
    id: string;
    node_type: string;
    label: string;
    status: string;
    metrics: {
      rps: number;
      p99_latency_ms: number;
      active_connections: number;
      error_rate: number;
    };
    metadata: Record<string, string>;
  }

  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedNode = $state<TopologyNodeData | null>(null);
  let isDrawerOpen = $state(false);
  let lastUpdated = $state<string>('');

  async function fetchTopology() {
    try {
      const res = await fetch('/api/v1/topology/graph');
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();

      let backendIndex = 0;
      let channelIndex = 0;

      const flowNodes: Node[] = (data.nodes || []).map((n: TopologyNodeData) => {
        let x = 60;
        let y = 160;

        if (n.node_type === 'client') {
          x = 60;
          y = 160;
        } else if (n.node_type === 'ingress') {
          x = 340;
          y = 160;
        } else if (n.node_type === 'filter') {
          x = 620;
          y = 160;
        } else if (n.node_type === 'router') {
          x = 900;
          y = 160;
        } else if (n.node_type === 'backend') {
          x = 1200;
          y = 60 + backendIndex * 130;
          backendIndex++;
        } else if (n.node_type === 'channel') {
          if (n.id.includes('sync')) {
            x = 340;
            y = 340;
          } else if (n.id.includes('siem')) {
            x = 620;
            y = 340;
          } else if (n.id.includes('secrets')) {
            x = 900;
            y = 340;
          } else {
            x = 1200;
            y = 340 + channelIndex * 130;
            channelIndex++;
          }
        }

        return {
          id: n.id,
          type: 'custom',
          position: { x, y },
          data: {
            ...n,
          },
        };
      });

      const flowEdges: Edge[] = (data.edges || []).map((e: any) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        label: e.label,
        animated: e.animated ?? true,
        style: e.status === 'tripped' ? 'stroke: #f43f5e; stroke-width: 2px;' : 'stroke: #3b82f6; stroke-width: 1.5px;',
      }));

      nodes = flowNodes;
      edges = flowEdges;
      lastUpdated = new Date().toLocaleTimeString();
      error = null;
    } catch (err: any) {
      error = err.message || 'Failed to fetch live topology graph';
    } finally {
      loading = false;
    }
  }

  function handleNodeClick({ detail: { node } }: any) {
    if (node && node.data) {
      selectedNode = node.data as TopologyNodeData;
      isDrawerOpen = true;
    }
  }

  function closeDrawer() {
    isDrawerOpen = false;
    selectedNode = null;
  }

  onMount(() => {
    fetchTopology();
    const interval = setInterval(fetchTopology, 3000);
    return () => clearInterval(interval);
  });
</script>

<div class="relative h-[calc(100vh-140px)] w-full flex flex-col bg-background rounded-lg border border-border overflow-hidden">
  <!-- Top Header Bar -->
  <div class="h-12 border-b border-border bg-card/60 px-4 flex items-center justify-between z-10 shrink-0">
    <div class="flex items-center gap-2">
      <Sliders class="h-4 w-4 text-primary" />
      <span class="font-semibold text-xs tracking-tight">Interactive Topology Flow (Svelte Flow)</span>
      <span class="text-[10px] text-muted-foreground uppercase tracking-wider px-1.5 py-0.5 rounded border border-border bg-muted/30">
        Live DAG
      </span>
    </div>

    <div class="flex items-center gap-3 text-xs">
      {#if lastUpdated}
        <span class="text-[11px] text-muted-foreground font-mono">Sync: {lastUpdated}</span>
      {/if}
      <button
        onclick={fetchTopology}
        disabled={loading}
        class="flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded border border-border bg-card hover:bg-muted transition-colors"
      >
        <RefreshCw class="h-3 w-3 {loading ? 'animate-spin' : ''}" />
        Refresh
      </button>
    </div>
  </div>

  <!-- Svelte Flow Canvas -->
  <div class="flex-1 w-full relative">
    {#if error}
      <div class="absolute inset-0 flex items-center justify-center bg-background/80 z-20">
        <div class="p-4 rounded-lg border border-destructive bg-destructive/10 text-destructive text-xs max-w-md">
          {error}
        </div>
      </div>
    {/if}

    <SvelteFlow
      {nodes}
      {edges}
      {nodeTypes}
      fitView
      on:nodeclick={handleNodeClick}
      class="bg-dot-pattern"
    >
      <Controls class="!bg-card !border-border !shadow-sm" />
      <Background gap={16} size={1} />
      <MiniMap
        class="!bg-card !border-border !shadow-sm"
        nodeStrokeWidth={2}
      />
    </SvelteFlow>
  </div>

  <!-- Slide-Out Node Inspector Drawer -->
  {#if isDrawerOpen && selectedNode}
    <div class="absolute right-0 top-12 bottom-0 w-80 border-l border-border bg-card/95 backdrop-blur p-4 shadow-xl z-30 flex flex-col justify-between overflow-y-auto">
      <div>
        <div class="flex items-center justify-between pb-3 border-b border-border mb-3">
          <div class="flex items-center gap-2">
            <span class="text-[10px] font-semibold uppercase px-1.5 py-0.5 rounded border bg-primary/10 text-primary border-primary/30">
              {selectedNode.node_type}
            </span>
            <span class="text-xs font-bold truncate max-w-[150px]">{selectedNode.label}</span>
          </div>
          <button
            onclick={closeDrawer}
            class="h-6 w-6 rounded hover:bg-muted flex items-center justify-center text-muted-foreground"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- Node Status -->
        <div class="space-y-3">
          <div>
            <div class="text-[11px] text-muted-foreground mb-1">Health Status</div>
            <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-semibold {selectedNode.status === 'healthy' ? 'bg-emerald-500/10 text-emerald-500' : 'bg-rose-500/10 text-rose-500'}">
              {#if selectedNode.status === 'healthy'}
                <CheckCircle2 class="h-3 w-3" />
                <span>OPERATIONAL (CLOSED)</span>
              {:else}
                <AlertTriangle class="h-3 w-3" />
                <span>CIRCUIT TRIPPED (OPEN)</span>
              {/if}
            </div>
          </div>

          <!-- Live Metrics Grid -->
          <div class="text-[11px] text-muted-foreground">Live Telemetry</div>
          <div class="grid grid-cols-2 gap-2 text-xs font-mono">
            <div class="p-2 rounded border border-border bg-muted/20">
              <div class="text-[10px] text-muted-foreground font-sans">Throughput</div>
              <div class="font-bold">{selectedNode.metrics.rps.toFixed(1)} RPS</div>
            </div>
            <div class="p-2 rounded border border-border bg-muted/20">
              <div class="text-[10px] text-muted-foreground font-sans">P99 Latency</div>
              <div class="font-bold">{selectedNode.metrics.p99_latency_ms.toFixed(2)}ms</div>
            </div>
            <div class="p-2 rounded border border-border bg-muted/20">
              <div class="text-[10px] text-muted-foreground font-sans">Connections</div>
              <div class="font-bold">{selectedNode.metrics.active_connections}</div>
            </div>
            <div class="p-2 rounded border border-border bg-muted/20">
              <div class="text-[10px] text-muted-foreground font-sans">Error Rate</div>
              <div class="font-bold">{(selectedNode.metrics.error_rate * 100).toFixed(1)}%</div>
            </div>
          </div>

          <!-- Metadata -->
          {#if selectedNode.metadata && Object.keys(selectedNode.metadata).length > 0}
            <div class="text-[11px] text-muted-foreground pt-2">Node Attributes</div>
            <div class="space-y-1.5 text-xs font-mono">
              {#each Object.entries(selectedNode.metadata) as [key, val]}
                <div class="flex items-center justify-between p-1.5 rounded bg-muted/30 border border-border text-[11px]">
                  <span class="text-muted-foreground">{key}:</span>
                  <span class="font-semibold truncate max-w-[140px]">{val}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Remediation Actions -->
      <div class="pt-4 border-t border-border mt-4">
        <button
          onclick={() => {
            fetchTopology();
            closeDrawer();
          }}
          class="w-full py-1.5 rounded bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition-colors shadow-sm flex items-center justify-center gap-1.5"
        >
          <Zap class="h-3.5 w-3.5" />
          Re-test Health Probe
        </button>
      </div>
    </div>
  {/if}
</div>
