<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, X, ShieldAlert, KeyRound, RefreshCw } from 'lucide-svelte';

  interface HitlTask {
    ticketId: string;
    requestTime: string;
    agent: string;
    action: string;
    riskTier: string;
    payload: string;
    hmacSignature: string;
  }

  let pendingTasks = $state<HitlTask[]>([]);
  let loading = $state(true);
  let actionMessage = $state<string | null>(null);

  async function fetchQueue() {
    try {
      const res = await fetch('/api/v1/hitl/queue');
      if (res.ok) {
        pendingTasks = await res.json();
      }
    } catch {
      // Retain existing state during network jitter
    } finally {
      loading = false;
    }
  }

  async function handleDecision(task: HitlTask, decision: 'APPROVED' | 'REJECTED') {
    try {
      const res = await fetch('/api/v1/hitl/resolve', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ticket_id: task.ticketId,
          signature: task.hmacSignature,
          approved: decision === 'APPROVED',
        }),
      });

      if (res.ok) {
        actionMessage = `Ticket ${task.ticketId} successfully ${decision.toLowerCase()}.`;
        pendingTasks = pendingTasks.filter((t) => t.ticketId !== task.ticketId);
        setTimeout(() => (actionMessage = null), 4000);
      } else {
        const err = await res.json();
        actionMessage = `Resolution error: ${err.error || 'Request failed'}`;
      }
    } catch (e: any) {
      actionMessage = `Network error: ${e.message}`;
    }
  }

  onMount(() => {
    fetchQueue();
    const interval = setInterval(fetchQueue, 3000);
    return () => clearInterval(interval);
  });
</script>

<div class="space-y-4 font-sans">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Suspended Task Human-in-the-Loop (HITL) Queue</h3>
      <p class="text-xs text-muted-foreground">High-risk actions intercepted before execution. Approvals require cryptographic HMAC verification.</p>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={fetchQueue}
        class="inline-flex items-center gap-1.5 rounded border border-border bg-muted/50 px-2.5 py-1 text-xs font-medium text-foreground hover:bg-muted transition-colors"
      >
        <RefreshCw class="h-3 w-3 shrink-0 {loading ? 'animate-spin' : ''}" />
        Refresh
      </button>
    </div>
  </div>

  {#if actionMessage}
    <div class="rounded-lg border border-primary/30 bg-primary/10 p-3 text-xs font-medium text-primary">
      {actionMessage}
    </div>
  {/if}

  {#if pendingTasks.length === 0}
    <div class="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground font-sans">
      No suspended tasks awaiting manual authorization. All agent execution streams clear.
    </div>
  {:else}
    <div class="grid grid-cols-1 gap-4">
      {#each pendingTasks as task (task.ticketId)}
        <div class="rounded-lg border border-border bg-card p-4 space-y-3 font-sans text-xs shadow-xs">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-border pb-3">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="rounded bg-rose-500/10 border border-rose-500/20 px-1.5 py-0.5 text-[10px] font-bold text-rose-500">
                {task.riskTier}
              </span>
              <span class="font-semibold text-foreground">{task.ticketId}</span>
              <span class="text-muted-foreground tabular-nums">({task.requestTime})</span>
            </div>
            <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground font-mono">
              <KeyRound class="h-3 w-3 shrink-0" />
              <span class="truncate max-w-[200px] sm:max-w-none">HMAC: {task.hmacSignature}</span>
            </div>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
            <div>
              <span class="text-muted-foreground">Requesting Agent:</span>
              <span class="ml-1 font-semibold text-foreground break-all">{task.agent}</span>
            </div>
            <div>
              <span class="text-muted-foreground">Intercepted Action:</span>
              <span class="ml-1 text-primary font-semibold break-all">{task.action}</span>
            </div>
          </div>

          <div class="rounded bg-muted/40 p-2.5 text-[11px] font-mono text-muted-foreground overflow-x-auto break-all whitespace-pre-wrap border border-border">
            {task.payload}
          </div>

          <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-end gap-2 pt-1">
            <button
              onclick={() => handleDecision(task, 'REJECTED')}
              class="inline-flex items-center justify-center gap-1.5 rounded border border-border bg-muted/60 px-3 py-2 sm:py-1.5 text-xs font-medium text-foreground hover:bg-muted transition-colors"
            >
              <X class="h-3.5 w-3.5 text-rose-500 shrink-0" />
              Reject Execution
            </button>
            <button
              onclick={() => handleDecision(task, 'APPROVED')}
              class="inline-flex items-center justify-center gap-1.5 rounded bg-primary text-primary-foreground px-3 py-2 sm:py-1.5 text-xs font-semibold hover:bg-primary/90 transition-colors shadow-xs"
            >
              <Check class="h-3.5 w-3.5 shrink-0" />
              Approve with HMAC
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
