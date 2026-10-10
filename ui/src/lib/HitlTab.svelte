<script lang="ts">
  import { AlertTriangle, Check, X, ShieldAlert, KeyRound } from 'lucide-svelte';

  let pendingTasks = $state([
    {
      ticketId: 'hitl_tkt_8091',
      requestTime: '18:41:58',
      agent: 'agent.finance.reconciler',
      action: 'transfer_funds',
      riskTier: 'CRITICAL',
      payload: '{"recipient_iban": "DE89370400440532013000", "amount_eur": 125000.00}',
      hmacSignature: 'sha256:d8a9f012...890c',
    },
    {
      ticketId: 'hitl_tkt_8092',
      requestTime: '18:38:22',
      agent: 'agent.devops.deployer',
      action: 'drop_database_table',
      riskTier: 'HIGH',
      payload: '{"target_table": "customers_staging", "cascade": true}',
      hmacSignature: 'sha256:44b1c90a...11ef',
    },
  ]);

  function handleDecision(ticketId: string, decision: 'APPROVED' | 'REJECTED') {
    pendingTasks = pendingTasks.filter((t) => t.ticketId !== ticketId);
  }
</script>

<div class="space-y-4">
  <div class="rounded-lg border border-border bg-card p-4">
    <h3 class="text-sm font-semibold tracking-tight">Suspended Task Human-in-the-Loop (HITL) Queue</h3>
    <p class="text-xs text-muted-foreground">High-risk actions intercepted before execution. Approvals require cryptographic HMAC verification.</p>
  </div>

  {#if pendingTasks.length === 0}
    <div class="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground font-mono">
      No suspended tasks awaiting manual authorization. All agent execution streams clear.
    </div>
  {:else}
    <div class="grid grid-cols-1 gap-4">
      {#each pendingTasks as task (task.ticketId)}
        <div class="rounded-lg border border-border bg-card p-4 space-y-3 font-mono text-xs">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-border pb-3">
            <div class="flex items-center gap-2">
              <span class="rounded bg-rose-500/10 border border-rose-500/20 px-1.5 py-0.5 text-[10px] font-bold text-rose-500">
                {task.riskTier}
              </span>
              <span class="font-semibold text-foreground">{task.ticketId}</span>
              <span class="text-muted-foreground">({task.requestTime})</span>
            </div>
            <div class="flex items-center gap-2 text-[10px] text-muted-foreground">
              <KeyRound class="h-3 w-3" />
              <span>HMAC: {task.hmacSignature}</span>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
            <div>
              <span class="text-muted-foreground">Requesting Agent:</span>
              <span class="ml-2 font-medium text-foreground">{task.agent}</span>
            </div>
            <div>
              <span class="text-muted-foreground">Intercepted Action:</span>
              <span class="ml-2 text-primary font-semibold">{task.action}</span>
            </div>
          </div>

          <div class="rounded bg-muted/40 p-2.5 text-[11px] font-mono text-muted-foreground overflow-x-auto border border-border">
            {task.payload}
          </div>

          <div class="flex items-center justify-end gap-2 pt-1">
            <button
              onclick={() => handleDecision(task.ticketId, 'REJECTED')}
              class="inline-flex items-center gap-1.5 rounded border border-border bg-muted/60 px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted transition-colors"
            >
              <X class="h-3.5 w-3.5 text-rose-500" />
              Reject Execution
            </button>
            <button
              onclick={() => handleDecision(task.ticketId, 'APPROVED')}
              class="inline-flex items-center gap-1.5 rounded bg-primary text-primary-foreground px-3 py-1.5 text-xs font-medium hover:bg-primary/90 transition-colors"
            >
              <Check class="h-3.5 w-3.5" />
              Approve with HMAC
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
