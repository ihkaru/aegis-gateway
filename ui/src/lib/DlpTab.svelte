<script lang="ts">
  import { onMount } from 'svelte';
  import { ShieldAlert, FileText, Lock, Filter, RefreshCw } from 'lucide-svelte';

  interface DlpEvent {
    id: string;
    time: string;
    policy: string;
    pattern: string;
    action: string;
    originalSample: string;
    destination: string;
  }

  let dlpEvents = $state<DlpEvent[]>([]);
  let loading = $state(true);

  async function fetchDlpEvents() {
    try {
      const res = await fetch('/api/v1/dlp/events');
      if (res.ok) {
        dlpEvents = await res.json();
      }
    } catch {
      // Retain state during network jitter
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchDlpEvents();
    const interval = setInterval(fetchDlpEvents, 3000);
    return () => clearInterval(interval);
  });
</script>

<div class="space-y-4 font-sans">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Bidirectional Data Loss Prevention (DLP) Stream</h3>
      <p class="text-xs text-muted-foreground">Inline sub-millisecond pattern redaction protecting LLM context windows.</p>
    </div>
    <div class="flex items-center gap-2 text-xs font-medium flex-wrap">
      <span class="rounded border border-border bg-muted/50 px-2 py-1 flex items-center gap-1.5 text-muted-foreground">
        <Filter class="h-3 w-3 shrink-0" />
        Filter: All Rules
      </span>
      <span class="rounded border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 text-emerald-500 font-semibold">
        Active Engine: Sub-ms FastMatcher
      </span>
      <button
        onclick={fetchDlpEvents}
        class="rounded border border-border bg-muted/50 p-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>

  {#if dlpEvents.length === 0}
    <div class="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground font-sans">
      No sensitive data leakage detected. DLP fast matcher is active and inspecting all input/output tokens.
    </div>
  {:else}
    <div class="rounded-lg border border-border bg-card overflow-hidden shadow-xs">
      <div class="overflow-x-auto w-full">
        <table class="w-full text-left text-xs min-w-[580px]">
          <thead class="bg-muted/40 text-muted-foreground uppercase text-[10px] tracking-wider border-b border-border">
            <tr>
              <th class="p-3 whitespace-nowrap">Event ID</th>
              <th class="p-3 whitespace-nowrap">Timestamp</th>
              <th class="p-3">Compliance Standard</th>
              <th class="p-3">Detected Pattern</th>
              <th class="p-3">Sanitized Sample</th>
              <th class="p-3 whitespace-nowrap">Action Enforced</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each dlpEvents as evt}
              <tr class="hover:bg-muted/30 transition-colors">
                <td class="p-3 text-muted-foreground tabular-nums whitespace-nowrap font-medium">{evt.id}</td>
                <td class="p-3 text-muted-foreground tabular-nums whitespace-nowrap">{evt.time || 'Live'}</td>
                <td class="p-3 font-semibold text-foreground">{evt.policy}</td>
                <td class="p-3 text-primary font-medium">{evt.pattern}</td>
                <td class="p-3 text-muted-foreground font-mono tabular-nums break-all text-[11px]">{evt.originalSample || '***'}</td>
                <td class="p-3 whitespace-nowrap">
                  {#if evt.action.startsWith('REDACTED')}
                    <span class="inline-flex items-center rounded border border-amber-500/20 bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-medium text-amber-500">
                      {evt.action}
                    </span>
                  {:else if evt.action.startsWith('BLOCKED')}
                    <span class="inline-flex items-center rounded border border-rose-500/20 bg-rose-500/10 px-1.5 py-0.5 text-[10px] font-medium text-rose-500">
                      {evt.action}
                    </span>
                  {:else}
                    <span class="inline-flex items-center rounded border border-emerald-500/20 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-500">
                      {evt.action}
                    </span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>
