<script lang="ts">
  import { onMount } from 'svelte';
  import { Server, Plus, Trash2, CheckCircle2, AlertCircle, RefreshCw, Terminal, Globe } from 'lucide-svelte';

  interface BackendItem {
    name: string;
    backend_type: string;
    target: string;
    status: string;
    tool_count: number;
    latency_ms: number;
  }

  let backends = $state<BackendItem[]>([]);
  let loading = $state(true);
  let showModal = $state(false);
  let actionMessage = $state<string | null>(null);

  // New Backend Form State
  let newName = $state('');
  let newType = $state<'Subprocess' | 'RemoteHttp'>('Subprocess');
  let newTarget = $state('');

  async function fetchBackends() {
    try {
      const res = await fetch('/api/v1/backends');
      if (res.ok) {
        backends = await res.json();
      }
    } catch {
      // Retain state
    } finally {
      loading = false;
    }
  }

  async function registerBackend() {
    if (!newName.trim() || !newTarget.trim()) return;
    try {
      const payload: any = {
        name: newName.trim(),
        backend_type: newType,
      };
      if (newType === 'Subprocess') {
        const parts = newTarget.trim().split(' ');
        payload.command = parts[0];
        payload.args = parts.slice(1);
      } else {
        payload.url = newTarget.trim();
      }

      const res = await fetch('/api/v1/backends', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (res.ok) {
        actionMessage = `Backend "${newName}" successfully registered!`;
        showModal = false;
        newName = '';
        newTarget = '';
        await fetchBackends();
        setTimeout(() => (actionMessage = null), 4000);
      } else {
        const err = await res.json();
        actionMessage = `Error: ${err.error || 'Failed to register'}`;
      }
    } catch (e: any) {
      actionMessage = `Network error: ${e.message}`;
    }
  }

  async function deleteBackend(name: string) {
    try {
      const res = await fetch(`/api/v1/backends/${encodeURIComponent(name)}`, {
        method: 'DELETE',
      });
      if (res.ok) {
        actionMessage = `Backend "${name}" deregistered successfully.`;
        backends = backends.filter((b) => b.name !== name);
        setTimeout(() => (actionMessage = null), 4000);
      }
    } catch (e: any) {
      actionMessage = `Deregister error: ${e.message}`;
    }
  }

  onMount(() => {
    fetchBackends();
  });
</script>

<div class="space-y-4 font-sans">
  <div class="rounded-lg border border-border bg-card p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 shadow-xs">
    <div>
      <h3 class="text-sm font-semibold tracking-tight">Upstream Backend Topology & Lifecycle</h3>
      <p class="text-xs text-muted-foreground">Manage subprocess and remote HTTP MCP servers dynamically without gateway restarts.</p>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={() => (showModal = true)}
        class="inline-flex items-center gap-1.5 rounded bg-primary text-primary-foreground px-2.5 py-1 text-xs font-medium hover:bg-primary/90 transition-colors shadow-xs"
      >
        <Plus class="h-3.5 w-3.5 shrink-0" />
        Register Backend
      </button>
      <button
        onclick={fetchBackends}
        class="rounded border border-border bg-muted/50 p-1 text-muted-foreground hover:text-foreground transition-colors"
        title="Refresh"
      >
        <RefreshCw class="h-3.5 w-3.5 shrink-0 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>

  {#if actionMessage}
    <div class="rounded-lg border border-primary/30 bg-primary/10 p-3 text-xs font-medium text-primary">
      {actionMessage}
    </div>
  {/if}

  {#if showModal}
    <div class="rounded-lg border border-border bg-card p-4 space-y-3 shadow-sm">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Add New MCP Backend</h4>
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
        <div>
          <label for="name-input" class="block font-medium mb-1">Backend Identifier</label>
          <input
            id="name-input"
            type="text"
            bind:value={newName}
            placeholder="e.g. pg-finance or search-node"
            class="w-full rounded border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
        <div>
          <label for="type-select" class="block font-medium mb-1">Type</label>
          <select
            id="type-select"
            bind:value={newType}
            class="w-full rounded border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          >
            <option value="Subprocess">Local Subprocess (Stdio)</option>
            <option value="RemoteHttp">Remote HTTP / SSE</option>
          </select>
        </div>
        <div>
          <label for="target-input" class="block font-medium mb-1">{newType === 'Subprocess' ? 'Command Line' : 'Endpoint URL'}</label>
          <input
            id="target-input"
            type="text"
            bind:value={newTarget}
            placeholder={newType === 'Subprocess' ? 'e.g. npx -y @modelcontextprotocol/server-postgres' : 'https://api.internal.corp/mcp'}
            class="w-full rounded border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
      </div>
      <div class="flex items-center justify-end gap-2 pt-2">
        <button
          onclick={() => (showModal = false)}
          class="rounded border border-border px-3 py-1 text-xs font-medium hover:bg-muted"
        >
          Cancel
        </button>
        <button
          onclick={registerBackend}
          class="rounded bg-primary text-primary-foreground px-3 py-1 text-xs font-medium hover:bg-primary/90"
        >
          Save & Connect
        </button>
      </div>
    </div>
  {/if}

  {#if backends.length === 0}
    <div class="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground">
      No backends registered in runtime topology. Click "Register Backend" to add your first MCP upstream server.
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
      {#each backends as b (b.name)}
        <div class="rounded-lg border border-border bg-card p-4 space-y-3 text-xs shadow-xs hover:border-muted-foreground/30 transition-colors">
          <div class="flex items-start justify-between gap-2">
            <div class="flex items-center gap-2">
              {#if b.backend_type === 'Subprocess'}
                <Terminal class="h-4 w-4 text-primary shrink-0" />
              {:else}
                <Globe class="h-4 w-4 text-emerald-500 shrink-0" />
              {/if}
              <div>
                <div class="font-semibold text-foreground text-sm">{b.name}</div>
                <div class="text-[10px] text-muted-foreground uppercase tracking-wide">{b.backend_type}</div>
              </div>
            </div>
            <button
              onclick={() => deleteBackend(b.name)}
              class="text-muted-foreground hover:text-rose-500 transition-colors p-1"
              title="Deregister Backend"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>

          <div class="p-2 rounded bg-muted/30 font-mono text-[11px] text-muted-foreground break-all">
            {b.target}
          </div>

          <div class="flex items-center justify-between text-[11px] pt-1 border-t border-border">
            <div class="flex items-center gap-1.5 text-emerald-500 font-medium">
              <span class="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
              <span>{b.status}</span>
            </div>
            <div class="tabular-nums text-muted-foreground">
              {b.tool_count} Tools | {b.latency_ms.toFixed(1)}ms
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
