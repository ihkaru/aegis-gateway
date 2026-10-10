<script lang="ts">
  import { ShieldAlert, FileText, Lock, Filter } from 'lucide-svelte';

  const dlpEvents = [
    {
      id: 'dlp_evt_1092',
      time: '18:39:12.401',
      policy: 'PCI-DSS v4.0 Requirement 3.4',
      pattern: 'Primary Account Number (PAN)',
      action: 'REDACTED_LUHN_MASK',
      originalSample: '4532-****-****-8910',
      destination: 'context_window_llm',
    },
    {
      id: 'dlp_evt_1091',
      time: '18:35:50.119',
      policy: 'HIPAA Safe Harbor PHI',
      pattern: 'Medical Record Number (MRN)',
      action: 'HASH_CHAINED_ENCRYPT',
      originalSample: 'MRN-****-8841',
      destination: 'tool_output_response',
    },
    {
      id: 'dlp_evt_1090',
      time: '18:28:11.884',
      policy: 'Infisical Secret Isolation',
      pattern: 'AWS_SECRET_ACCESS_KEY',
      action: 'BLOCKED_EGRESS',
      originalSample: 'AKIA****************',
      destination: 'external_egress_socket',
    },
  ];
</script>

<div class="space-y-4">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Bidirectional Data Loss Prevention (DLP) Stream</h3>
      <p class="text-xs text-muted-foreground">Inline sub-millisecond regex & Presidio pattern redaction protecting LLM context windows.</p>
    </div>
    <div class="flex items-center gap-2 text-xs font-mono">
      <span class="rounded border border-border bg-muted/50 px-2 py-1 flex items-center gap-1.5 text-muted-foreground">
        <Filter class="h-3 w-3" />
        Filter: All Rules
      </span>
      <span class="rounded border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 text-emerald-500 font-medium">
        Active Engine: Sub-ms FastMatcher
      </span>
    </div>
  </div>

  <div class="rounded-lg border border-border bg-card overflow-hidden">
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead class="bg-muted/50 text-muted-foreground uppercase text-[10px] tracking-wider border-b border-border">
          <tr>
            <th class="p-3">Event ID</th>
            <th class="p-3">Timestamp</th>
            <th class="p-3">Compliance Standard</th>
            <th class="p-3">Detected Pattern</th>
            <th class="p-3">Sanitized Representation</th>
            <th class="p-3">Action Enforced</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border">
          {#each dlpEvents as evt}
            <tr class="hover:bg-muted/30 transition-colors">
              <td class="p-3 text-muted-foreground">{evt.id}</td>
              <td class="p-3 text-muted-foreground">{evt.time}</td>
              <td class="p-3 font-semibold text-foreground">{evt.policy}</td>
              <td class="p-3 text-primary">{evt.pattern}</td>
              <td class="p-3 text-muted-foreground">{evt.originalSample}</td>
              <td class="p-3">
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
</div>
