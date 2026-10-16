<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Search,
    Key,
    ShieldCheck,
    RefreshCw,
    Plus,
    Trash2,
    ToggleLeft,
    ToggleRight,
  } from 'lucide-svelte';
  import SearchPlayground from './SearchPlayground.svelte';

  interface KeyItem {
    id: string;
    masked_key: string;
    label: string;
    active: boolean;
    usage_count: number;
    failure_count: number;
    last_used_at: string | null;
    in_cooldown: boolean;
  }

  interface SearchResponse {
    provider: string;
    strategy: string;
    total_keys: number;
    active_keys: number;
    keys: KeyItem[];
  }

  let loading = $state(true);
  let keysData = $state<SearchResponse | null>(null);
  let newKeyInput = $state('');
  let newKeyLabel = $state('');
  let addingKey = $state(false);

  async function fetchKeys() {
    loading = true;
    try {
      const res = await fetch('/api/v1/search/keys');
      if (res.ok) {
        keysData = await res.json();
      }
    } catch {
      // Retain previous state
    } finally {
      loading = false;
    }
  }

  async function handleAddKey() {
    if (!newKeyInput.trim()) return;
    addingKey = true;
    try {
      const res = await fetch('/api/v1/search/keys', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key: newKeyInput.trim(), label: newKeyLabel.trim() || undefined }),
      });
      if (res.ok) {
        newKeyInput = '';
        newKeyLabel = '';
        await fetchKeys();
      } else {
        const err = await res.json();
        alert('Gagal menambah kunci: ' + (err.error || 'Server error'));
      }
    } finally {
      addingKey = false;
    }
  }

  async function handleRevokeKey(id: string) {
    if (!confirm('Apakah Anda yakin ingin mencabut (revoke) kunci ini dari pool?')) return;
    try {
      const res = await fetch(`/api/v1/search/keys/${id}`, { method: 'DELETE' });
      if (res.ok) {
        await fetchKeys();
      }
    } catch {
      alert('Gagal mencabut kunci');
    }
  }

  async function handleToggleActive(key: KeyItem) {
    try {
      const res = await fetch(`/api/v1/search/keys/${key.id}/toggle`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ active: !key.active }),
      });
      if (res.ok) {
        await fetchKeys();
      }
    } catch {
      alert('Gagal memperbarui status kunci');
    }
  }

  async function handleUpdateStrategy(strategy: 'round_robin' | 'least_used') {
    try {
      const res = await fetch('/api/v1/search/strategy', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ strategy }),
      });
      if (res.ok) {
        await fetchKeys();
      }
    } catch {
      alert('Gagal mengubah strategi pool');
    }
  }

  onMount(() => {
    fetchKeys();
  });
</script>

<div class="space-y-6">
  <!-- Header & KPI Cards -->
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
    <div>
      <h2 class="text-xl font-bold tracking-tight [overflow-wrap:anywhere] break-words">Manajemen Search Engine & Key Pool</h2>
      <p class="text-xs text-muted-foreground mt-0.5">Pool multi-kunci cerdas untuk Web Search dengan failover 429 otomatis dan zero vendor lock-in.</p>
    </div>
    <div class="flex items-center gap-2">
      <button onclick={fetchKeys} class="h-9 px-3 rounded-lg border border-border bg-card text-xs flex items-center gap-1.5 hover:bg-muted/50 cursor-pointer min-h-[44px]">
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        <span>Segarkan</span>
      </button>
    </div>
  </div>

  <!-- Metric Overview -->
  <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
    <div class="p-3.5 rounded-xl border border-border bg-card">
      <div class="text-[10px] text-muted-foreground uppercase font-medium">Provider Aktif</div>
      <div class="text-lg font-bold mt-1 text-primary uppercase font-mono">{keysData?.provider || 'EXA'}</div>
      <div class="text-[10px] text-muted-foreground mt-0.5">Interface-First Driver</div>
    </div>
    <div class="p-3.5 rounded-xl border border-border bg-card">
      <div class="text-[10px] text-muted-foreground uppercase font-medium">Total Kunci Terdaftar</div>
      <div class="text-lg font-bold mt-1 font-mono">{keysData?.total_keys ?? 0}</div>
      <div class="text-[10px] text-muted-foreground mt-0.5">Zero-Knowledge Storage</div>
    </div>
    <div class="p-3.5 rounded-xl border border-border bg-card">
      <div class="text-[10px] text-muted-foreground uppercase font-medium">Kunci Siap Eksekusi</div>
      <div class="text-lg font-bold mt-1 text-emerald-500 font-mono">{keysData?.active_keys ?? 0}</div>
      <div class="text-[10px] text-muted-foreground mt-0.5">Aktif & Non-Cooldown</div>
    </div>
    <div class="p-3.5 rounded-xl border border-border bg-card">
      <div class="text-[10px] text-muted-foreground uppercase font-medium">Strategi Load Balancing</div>
      <div class="flex items-center gap-1.5 mt-1">
        <select
          value={keysData?.strategy || 'round_robin'}
          onchange={(e) => handleUpdateStrategy((e.target as HTMLSelectElement).value as any)}
          class="h-7 text-xs bg-muted/60 border border-border rounded px-2 font-mono"
        >
          <option value="round_robin">Round-Robin</option>
          <option value="least_used">Least-Used</option>
        </select>
      </div>
      <div class="text-[10px] text-muted-foreground mt-0.5">Smart Fair-Share Routing</div>
    </div>
  </div>

  <!-- Add Key Section -->
  <div class="p-4 rounded-xl border border-border bg-card space-y-3">
    <div class="text-xs font-semibold flex items-center gap-1.5">
      <Plus class="w-4 h-4 text-primary" />
      <span>Tambah Kunci API ke Pool</span>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-12 gap-2">
      <input
        type="password"
        bind:value={newKeyInput}
        placeholder="Masukkan API Key (misal: b436b248-...)"
        class="md:col-span-7 h-9 px-3 rounded-lg border border-border bg-background text-xs font-mono"
      />
      <input
        type="text"
        bind:value={newKeyLabel}
        placeholder="Label / Alias (opsional, misal: Backup Exa #7)"
        class="md:col-span-3 h-9 px-3 rounded-lg border border-border bg-background text-xs"
      />
      <button
        onclick={handleAddKey}
        disabled={addingKey || !newKeyInput.trim()}
        class="md:col-span-2 h-9 px-3 rounded-lg bg-primary text-primary-foreground text-xs font-medium flex items-center justify-center gap-1.5 hover:opacity-90 disabled:opacity-50 cursor-pointer min-h-[44px]"
      >
        <Key class="w-3.5 h-3.5" />
        <span>{addingKey ? 'Menyimpan...' : 'Tambah Kunci'}</span>
      </button>
    </div>
  </div>

  <!-- Key Pool Table -->
  <div class="p-4 rounded-xl border border-border bg-card space-y-3">
    <div class="flex items-center justify-between">
      <div class="text-xs font-semibold flex items-center gap-1.5">
        <ShieldCheck class="w-4 h-4 text-emerald-500" />
        <span>Daftar Kunci dalam Pool ({keysData?.keys?.length ?? 0})</span>
      </div>
      <span class="text-[10px] text-muted-foreground">Otomatis Cooldown 60s saat HTTP 429</span>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full text-xs text-left">
        <thead>
          <tr class="border-b border-border/50 text-muted-foreground text-[10px] uppercase">
            <th class="py-2 px-3">Status</th>
            <th class="py-2 px-3">Label</th>
            <th class="py-2 px-3">Masked Key</th>
            <th class="py-2 px-3 text-right">Sukses</th>
            <th class="py-2 px-3 text-right">Gagal</th>
            <th class="py-2 px-3">Terakhir Digunakan</th>
            <th class="py-2 px-3 text-right">Aksi</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border/30">
          {#if !keysData?.keys || keysData.keys.length === 0}
            <tr>
              <td colspan="7" class="py-6 text-center text-muted-foreground text-xs">Belum ada kunci terdaftar di pool.</td>
            </tr>
          {:else}
            {#each keysData.keys as k (k.id)}
              <tr class="hover:bg-muted/20">
                <td class="py-2 px-3">
                  {#if !k.active}
                    <span class="px-2 py-0.5 rounded text-[10px] bg-muted text-muted-foreground">Nonaktif</span>
                  {:else if k.in_cooldown}
                    <span class="px-2 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-500 font-medium animate-pulse">Cooldown</span>
                  {:else}
                    <span class="px-2 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-500 font-medium">Aktif</span>
                  {/if}
                </td>
                <td class="py-2 px-3 font-medium">{k.label}</td>
                <td class="py-2 px-3 font-mono text-muted-foreground">{k.masked_key}</td>
                <td class="py-2 px-3 text-right font-mono text-emerald-500">{k.usage_count}</td>
                <td class="py-2 px-3 text-right font-mono {k.failure_count > 0 ? 'text-rose-500' : 'text-muted-foreground'}">{k.failure_count}</td>
                <td class="py-2 px-3 text-muted-foreground text-[10px] font-mono">
                  {k.last_used_at ? new Date(k.last_used_at).toLocaleTimeString() : '-'}
                </td>
                <td class="py-2 px-3 text-right">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      onclick={() => handleToggleActive(k)}
                      title={k.active ? 'Nonaktifkan' : 'Aktifkan'}
                      class="p-1 rounded hover:bg-muted/60 text-muted-foreground hover:text-foreground cursor-pointer min-h-[44px] min-w-[44px] flex items-center justify-center"
                    >
                      {#if k.active}
                        <ToggleRight class="w-4 h-4 text-emerald-500" />
                      {:else}
                        <ToggleLeft class="w-4 h-4 text-muted-foreground" />
                      {/if}
                    </button>
                    <button
                      onclick={() => handleRevokeKey(k.id)}
                      title="Cabut (Revoke)"
                      class="p-1 rounded hover:bg-rose-500/10 text-muted-foreground hover:text-rose-500 cursor-pointer min-h-[44px] min-w-[44px] flex items-center justify-center"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>

  <!-- Interactive Test Playground -->
  <SearchPlayground onSearchExecuted={fetchKeys} />
</div>
