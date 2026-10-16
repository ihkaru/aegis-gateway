<!-- SPDX-License-Identifier: MIT -->
<script lang="ts">
  import { Search, Play, AlertCircle, ExternalLink } from 'lucide-svelte';

  let { onSearchExecuted }: { onSearchExecuted?: () => void } = $props();

  let testQuery = $state('Badan Pusat Statistik Kabupaten Mempawah');
  let testLimit = $state(3);
  let testSearchType = $state('auto');
  let testing = $state(false);
  let testResult = $state<any>(null);
  let testError = $state<string | null>(null);

  async function handleRunTestSearch() {
    if (!testQuery.trim()) return;
    testing = true;
    testResult = null;
    testError = null;
    try {
      const res = await fetch('/api/v1/search/query', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: testQuery.trim(),
          limit: testLimit,
          search_type: testSearchType,
        }),
      });
      if (res.ok) {
        testResult = await res.json();
        onSearchExecuted?.();
      } else {
        const err = await res.json();
        testError = err.error || 'Pencarian gagal';
      }
    } catch (e: any) {
      testError = e.message || 'Network error';
    } finally {
      testing = false;
    }
  }
</script>

<div class="p-4 rounded-xl border border-border bg-card space-y-3">
  <div class="text-xs font-semibold flex items-center gap-1.5">
    <Search class="w-4 h-4 text-primary" />
    <span>Uji Eksekusi Live Web Search Melalui Pool</span>
  </div>
  <div class="grid grid-cols-1 sm:grid-cols-12 gap-2">
    <input
      type="text"
      bind:value={testQuery}
      placeholder="Kueri pencarian..."
      class="sm:col-span-7 h-9 px-3 rounded-lg border border-border bg-background text-xs"
    />
    <select bind:value={testSearchType} class="sm:col-span-2 h-9 px-2 rounded-lg border border-border bg-background text-xs font-mono">
      <option value="auto">Auto</option>
      <option value="neural">Neural</option>
      <option value="keyword">Keyword</option>
    </select>
    <input
      type="number"
      bind:value={testLimit}
      min="1"
      max="10"
      class="sm:col-span-1 h-9 px-2 rounded-lg border border-border bg-background text-xs text-center font-mono"
    />
    <button
      onclick={handleRunTestSearch}
      disabled={testing || !testQuery.trim()}
      class="sm:col-span-2 h-9 px-3 rounded-lg bg-emerald-600 text-white text-xs font-medium flex items-center justify-center gap-1.5 hover:bg-emerald-500 disabled:opacity-50 cursor-pointer min-h-[44px]"
    >
      <Play class="w-3.5 h-3.5 {testing ? 'animate-spin' : ''}" />
      <span>{testing ? 'Mencari...' : 'Eksekusi'}</span>
    </button>
  </div>

  {#if testError}
    <div class="p-3 rounded-lg bg-rose-500/10 border border-rose-500/20 text-rose-500 text-xs flex items-center gap-2">
      <AlertCircle class="w-4 h-4 shrink-0" />
      <span class="[overflow-wrap:anywhere] break-words">{testError}</span>
    </div>
  {/if}

  {#if testResult}
    <div class="p-3.5 rounded-lg bg-muted/40 border border-border space-y-2.5">
      <div class="flex items-center justify-between text-[11px] text-muted-foreground border-b border-border/40 pb-1.5">
        <span>Engine: <b class="text-foreground uppercase font-mono">{testResult.provider}</b></span>
        <span>Latency: <b class="text-foreground font-mono">{testResult.search_time_ms}ms</b></span>
        <span>Ditemukan: <b class="text-foreground font-mono">{testResult.results?.length ?? 0} hasil</b></span>
      </div>
      <div class="space-y-2">
        {#each testResult.results as item, idx}
          <div class="text-xs border-b border-border/20 pb-2 last:border-b-0">
            <a href={item.url} target="_blank" rel="noopener noreferrer" class="font-medium text-primary hover:underline flex items-center gap-1">
              <span>{idx + 1}. {item.title}</span>
              <ExternalLink class="w-3 h-3 inline shrink-0" />
            </a>
            <div class="text-[10px] text-muted-foreground font-mono mt-0.5">{item.url}</div>
            {#if item.highlights && item.highlights.length > 0}
              <div class="mt-1 pl-2 border-l-2 border-primary/40 text-[11px] text-muted-foreground italic space-y-0.5">
                {#each item.highlights as hl}
                  <div>"{hl}"</div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
