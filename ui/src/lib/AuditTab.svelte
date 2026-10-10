<script lang="ts">
  import { onMount } from 'svelte';
  import { FileText, CheckCircle2, Shield, Download, Hash, Clock, RefreshCw } from 'lucide-svelte';

  interface Soc2Report {
    report_id: string;
    generated_at: string;
    total_events_audited: number;
    hash_chain_root: string;
    compliance_controls: {
      control_id: string;
      title: string;
      status: string;
    }[];
  }

  let generating = $state(false);
  let report = $state<Soc2Report | null>(null);
  let message = $state<string | null>(null);

  async function generateSoc2Report() {
    generating = true;
    message = null;
    try {
      const res = await fetch('/api/v1/audit/soc2', {
        method: 'POST',
      });
      if (res.ok) {
        report = await res.json();
        message = 'SOC 2 Type II Non-Repudiation Report successfully generated with cryptographic SHA-256 hash chain!';
        setTimeout(() => (message = null), 5000);
      } else {
        const err = await res.json();
        message = `Report generation error: ${err.error || 'Failed'}`;
      }
    } catch (e: any) {
      message = `Network error: ${e.message}`;
    } finally {
      generating = false;
    }
  }

  function downloadReportJson() {
    if (!report) return;
    const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `soc2-report-${report.report_id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  onMount(() => {
    generateSoc2Report();
  });
</script>

<div class="space-y-4 font-sans text-xs">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">SOC 2 Type II Compliance & Non-Repudiation Audit Ledger</h3>
      <p class="text-xs text-muted-foreground">Tamper-evident SHA-256 cryptographic hash-chaining logging every invocation, refusal, and DLP modification.</p>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={generateSoc2Report}
        disabled={generating}
        class="inline-flex items-center gap-1.5 rounded bg-primary text-primary-foreground px-3 py-1.5 font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors shadow-xs"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0 {generating ? 'animate-spin' : ''}" />
        {generating ? 'Verifying Ledger...' : 'Generate SOC 2 Report'}
      </button>
    </div>
  </div>

  {#if message}
    <div class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs font-medium text-emerald-500">
      {message}
    </div>
  {/if}

  {#if report}
    <div class="space-y-4">
      <!-- Report Header Cards -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div class="rounded-lg border border-border bg-card p-3.5 space-y-1 shadow-xs">
          <div class="text-muted-foreground text-[11px] font-medium flex items-center gap-1.5">
            <Hash class="h-3.5 w-3.5 text-primary" />
            Audit Report ID
          </div>
          <div class="font-mono font-bold text-sm text-foreground truncate">{report.report_id}</div>
          <div class="text-[10px] text-muted-foreground">Certified Immutable</div>
        </div>

        <div class="rounded-lg border border-border bg-card p-3.5 space-y-1 shadow-xs">
          <div class="text-muted-foreground text-[11px] font-medium flex items-center gap-1.5">
            <CheckCircle2 class="h-3.5 w-3.5 text-emerald-500" />
            Events Audited
          </div>
          <div class="font-bold text-sm text-foreground tabular-nums">{report.total_events_audited} Records</div>
          <div class="text-[10px] text-emerald-500 font-medium">100% Hash-Chained</div>
        </div>

        <div class="rounded-lg border border-border bg-card p-3.5 space-y-1 shadow-xs">
          <div class="text-muted-foreground text-[11px] font-medium flex items-center gap-1.5">
            <Clock class="h-3.5 w-3.5 text-muted-foreground" />
            Generation Timestamp
          </div>
          <div class="font-mono text-xs text-foreground truncate">{report.generated_at}</div>
          <div class="text-[10px] text-muted-foreground">W3C Trace Context Linked</div>
        </div>
      </div>

      <!-- Cryptographic Hash Root -->
      <div class="rounded-lg border border-border bg-card p-4 space-y-2 shadow-xs">
        <div class="flex items-center justify-between">
          <span class="font-semibold text-foreground text-xs flex items-center gap-1.5">
            <Shield class="h-4 w-4 text-primary" />
            Cryptographic SHA-256 Hash Chain Root (Merkle Non-Repudiation)
          </span>
          <button
            onclick={downloadReportJson}
            class="inline-flex items-center gap-1 text-[11px] font-medium text-primary hover:underline"
          >
            <Download class="h-3 w-3" />
            Export JSON
          </button>
        </div>
        <div class="p-2.5 rounded bg-muted/40 font-mono text-[11px] text-foreground break-all border border-border">
          {report.hash_chain_root}
        </div>
      </div>

      <!-- Trust Services Criteria Status -->
      <div class="rounded-lg border border-border bg-card p-4 space-y-3 shadow-xs">
        <h4 class="font-semibold text-foreground text-xs">Trust Services Criteria Controls Verification</h4>
        <div class="divide-y divide-border border rounded border-border overflow-hidden">
          {#each report.compliance_controls as c}
            <div class="p-3 flex items-center justify-between gap-2 hover:bg-muted/30 transition-colors">
              <div class="space-y-0.5">
                <div class="font-semibold text-foreground text-xs">{c.control_id} - {c.title}</div>
                <div class="text-[11px] text-muted-foreground">Continuous automated policy and DLP verification gate</div>
              </div>
              <span class="rounded bg-emerald-500/10 border border-emerald-500/20 px-2 py-0.5 text-[10px] font-bold text-emerald-500 uppercase">
                {c.status}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
