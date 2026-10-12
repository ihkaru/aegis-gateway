<!-- SPDX-License-Identifier: MIT -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Key, ShieldCheck, Copy, Check, Trash2, Plus, Terminal, RefreshCw, AlertCircle } from 'lucide-svelte';

  interface PatItem {
    id: string;
    name: string;
    subject: string;
    created_at: number;
    expires_at: number;
    last_used_at: number | null;
    scopes: string[];
    is_active: boolean;
  }

  let pats = $state<PatItem[]>([]);
  let loading = $state(false);
  let error = $state('');

  // Form state
  let tokenName = $state('Claude Code / Cursor Token');
  let ttlDays = $state(30);
  let maxIdleDays = $state(14);
  let newlyCreatedToken = $state('');
  let copiedToken = $state(false);
  let copiedConfig = $state(false);

  async function fetchPats() {
    loading = true;
    error = '';
    try {
      const res = await fetch('/api/v1/auth/pat');
      if (res.ok) {
        pats = await res.json();
      } else {
        error = 'Gagal memuat daftar Personal Access Token.';
      }
    } catch (e: any) {
      error = e.message || 'Gagal tersambung ke gateway.';
    } finally {
      loading = false;
    }
  }

  async function handleCreatePat() {
    if (!tokenName.trim()) return;
    loading = true;
    error = '';
    try {
      const res = await fetch('/api/v1/auth/pat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: tokenName.trim(),
          ttl_days: ttlDays,
          max_idle_days: maxIdleDays,
          scopes: ['tools:execute:safe', 'execute_code', 'gateway_search_tools']
        })
      });
      if (res.ok) {
        const data = await res.json();
        newlyCreatedToken = data.token;
        await fetchPats();
      } else {
        const errData = await res.json().catch(() => ({}));
        error = errData.error_description || 'Gagal membuat PAT.';
      }
    } catch (e: any) {
      error = e.message || 'Network error.';
    } finally {
      loading = false;
    }
  }

  async function handleRevoke(id: string) {
    if (!confirm('Apakah Anda yakin ingin mencabut (revoke) token ini segera?')) return;
    try {
      const res = await fetch('/api/v1/auth/pat/revoke', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id })
      });
      if (res.ok) {
        await fetchPats();
      }
    } catch (e) {
      // ignore
    }
  }

  function copyText(txt: string, isJson: boolean = false) {
    navigator.clipboard.writeText(txt).then(() => {
      if (isJson) {
        copiedConfig = true;
        setTimeout(() => (copiedConfig = false), 2500);
      } else {
        copiedToken = true;
        setTimeout(() => (copiedToken = false), 2500);
      }
    });
  }

  function getClientConfigJson(token: string) {
    return JSON.stringify({
      mcpServers: {
        propylon: {
          url: "https://mcp.dvlpid.my.id/mcp",
          headers: {
            Authorization: `Bearer ${token}`
          }
        }
      }
    }, null, 2);
  }

  onMount(() => {
    fetchPats();
  });
</script>

<div class="space-y-6">
  <!-- Header Card -->
  <div class="rounded-xl border border-border bg-card p-5 shadow-xs">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div>
        <div class="flex items-center gap-2">
          <Key class="h-5 w-5 text-primary" />
          <h2 class="text-base font-semibold tracking-tight text-foreground">Personal Access Tokens (PAT)</h2>
          <span class="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">Zero-Install Host</span>
        </div>
        <p class="text-xs text-muted-foreground mt-1 max-w-2xl">
          Gunakan token ini langsung di Claude Code, Cursor, atau editor tanpa perlu menginstal binary apapun di perangkat host. Dilengkapi proteksi sliding-window 14 hari dan pembatasan scope least-privilege.
        </p>
      </div>
      <button
        onclick={fetchPats}
        disabled={loading}
        class="self-start sm:self-auto px-3 py-1.5 rounded-lg border border-border bg-muted/40 hover:bg-muted text-xs font-medium flex items-center gap-1.5 cursor-pointer transition-colors"
      >
        <RefreshCw class="h-3.5 w-3.5 {loading ? 'animate-spin' : ''}" />
        <span>Segarkan</span>
      </button>
    </div>
  </div>

  <!-- Generate Token Form -->
  <div class="rounded-xl border border-border bg-card p-5 space-y-4 shadow-xs">
    <h3 class="text-sm font-semibold flex items-center gap-2 text-foreground">
      <Plus class="h-4 w-4 text-primary" />
      <span>Buat Token Baru</span>
    </h3>

    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
      <div class="space-y-1 sm:col-span-1">
        <label for="pat-name-input" class="text-xs font-medium text-muted-foreground">Deskripsi / Nama Token</label>
        <input
          id="pat-name-input"
          type="text"
          bind:value={tokenName}
          placeholder="contoh: Laptop Claude Code"
          class="w-full bg-background border border-border rounded-lg px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary font-sans"
        />
      </div>
      <div class="space-y-1">
        <label for="pat-ttl-select" class="text-xs font-medium text-muted-foreground">Masa Berlaku (TTL)</label>
        <select
          id="pat-ttl-select"
          bind:value={ttlDays}
          class="w-full bg-background border border-border rounded-lg px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary"
        >
          <option value={30}>30 Hari (Direkomendasikan)</option>
          <option value={60}>60 Hari</option>
          <option value={90}>90 Hari (Maksimal CISO)</option>
        </select>
      </div>
      <div class="space-y-1">
        <label for="pat-idle-select" class="text-xs font-medium text-muted-foreground">Batas Inaktivitas (Sliding)</label>
        <select
          id="pat-idle-select"
          bind:value={maxIdleDays}
          class="w-full bg-background border border-border rounded-lg px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary"
        >
          <option value={7}>7 Hari Inaktif</option>
          <option value={14}>14 Hari Inaktif (Standar)</option>
          <option value={30}>30 Hari Inaktif</option>
        </select>
      </div>
    </div>

    <div class="flex items-center justify-between pt-2">
      <span class="text-[11px] text-muted-foreground flex items-center gap-1.5">
        <ShieldCheck class="h-3.5 w-3.5 text-emerald-400" />
        Otomatis diberikan hak akses: <code>tools:execute:safe</code> (operasi destructive dicegah).
      </span>
      <button
        onclick={handleCreatePat}
        disabled={loading || !tokenName.trim()}
        class="px-4 py-1.5 rounded-lg bg-primary text-primary-foreground font-medium text-xs flex items-center gap-1.5 hover:opacity-90 disabled:opacity-50 cursor-pointer shadow-xs transition-opacity"
      >
        <Key class="h-3.5 w-3.5" />
        <span>Generate Token</span>
      </button>
    </div>

    {#if error}
      <div class="p-3 rounded-lg border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs flex items-center gap-2">
        <AlertCircle class="h-4 w-4 shrink-0" />
        <span>{error}</span>
      </div>
    {/if}

    <!-- One-Time Token Display Modal / Banner -->
    {#if newlyCreatedToken}
      <div class="mt-4 p-4 rounded-xl border border-emerald-500/30 bg-emerald-500/5 space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-emerald-400 flex items-center gap-1.5">
            <Check class="h-4 w-4" /> Token Berhasil Dibuat (Simpan Sekarang)
          </span>
          <button
            onclick={() => (newlyCreatedToken = '')}
            class="text-[10px] text-muted-foreground hover:text-foreground"
          >
            Tutup
          </button>
        </div>

        <div class="flex items-center gap-2">
          <input
            readonly
            type="text"
            value={newlyCreatedToken}
            class="flex-1 bg-background border border-border rounded-lg px-3 py-1.5 text-xs font-mono text-emerald-400 focus:outline-none"
          />
          <button
            onclick={() => copyText(newlyCreatedToken, false)}
            class="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-medium text-xs flex items-center gap-1.5 cursor-pointer shadow-xs"
          >
            {#if copiedToken}
              <Check class="h-3.5 w-3.5" /> <span>Tersalin!</span>
            {:else}
              <Copy class="h-3.5 w-3.5" /> <span>Salin Token</span>
            {/if}
          </button>
        </div>

        <!-- Claude Code / Cursor Config Snippet -->
        <div class="pt-2 border-t border-emerald-500/20 space-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-[11px] font-medium text-muted-foreground flex items-center gap-1">
              <Terminal class="h-3 w-3" /> Konfigurasi <code>~/.claude.json</code> atau <code>~/.cursor/mcp.json</code>:
            </span>
            <button
              onclick={() => copyText(getClientConfigJson(newlyCreatedToken), true)}
              class="text-[10px] text-primary hover:underline flex items-center gap-1 font-mono"
            >
              {#if copiedConfig}
                <Check class="h-3 w-3 text-emerald-400" />
                <span class="text-emerald-400">JSON Tersalin!</span>
              {:else}
                <Copy class="h-3 w-3" />
                <span>Salin JSON Utuh</span>
              {/if}
            </button>
          </div>
          <pre class="p-2.5 rounded-lg bg-slate-950 border border-slate-800 text-[11px] font-mono text-slate-200 overflow-x-auto leading-relaxed">{getClientConfigJson(newlyCreatedToken)}</pre>
        </div>
      </div>
    {/if}
  </div>

  <!-- Active Tokens Table -->
  <div class="rounded-xl border border-border bg-card p-5 space-y-3 shadow-xs">
    <h3 class="text-sm font-semibold text-foreground">Daftar Personal Access Tokens Aktif</h3>

    {#if pats.length === 0 && !loading}
      <p class="text-xs text-muted-foreground py-4 text-center">Belum ada token yang dibuat. Buat token di formulir atas untuk memulai.</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-left text-xs border-collapse">
          <thead>
            <tr class="border-b border-border text-muted-foreground">
              <th class="py-2.5 px-3 font-medium">Nama Token</th>
              <th class="py-2.5 px-3 font-medium">Status</th>
              <th class="py-2.5 px-3 font-medium">Dibuat Pada</th>
              <th class="py-2.5 px-3 font-medium">Terakhir Dipakai</th>
              <th class="py-2.5 px-3 font-medium">Kedaluwarsa</th>
              <th class="py-2.5 px-3 font-medium text-right">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each pats as p (p.id)}
              <tr class="hover:bg-muted/30 transition-colors">
                <td class="py-2.5 px-3">
                  <div class="font-medium text-foreground">{p.name}</div>
                  <div class="font-mono text-[10px] text-muted-foreground truncate max-w-[180px]">{p.id}</div>
                </td>
                <td class="py-2.5 px-3">
                  {#if p.is_active}
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                      Aktif
                    </span>
                  {:else}
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
                      Dicabut / Habis
                    </span>
                  {/if}
                </td>
                <td class="py-2.5 px-3 text-muted-foreground tabular-nums">
                  {new Date(p.created_at * 1000).toLocaleDateString('id-ID')}
                </td>
                <td class="py-2.5 px-3 text-muted-foreground tabular-nums">
                  {p.last_used_at ? new Date(p.last_used_at * 1000).toLocaleDateString('id-ID') : 'Belum pernah'}
                </td>
                <td class="py-2.5 px-3 text-muted-foreground tabular-nums">
                  {new Date(p.expires_at * 1000).toLocaleDateString('id-ID')}
                </td>
                <td class="py-2.5 px-3 text-right">
                  {#if p.is_active}
                    <button
                      onclick={() => handleRevoke(p.id)}
                      class="px-2 py-1 rounded border border-rose-500/30 text-rose-400 hover:bg-rose-500/10 text-[11px] font-medium transition cursor-pointer flex items-center gap-1 ml-auto"
                      title="Cabut token ini segera"
                    >
                      <Trash2 class="h-3 w-3" />
                      <span>Cabut</span>
                    </button>
                  {:else}
                    <span class="text-[10px] text-muted-foreground">Tidak aktif</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>
