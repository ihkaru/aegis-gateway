<!-- SPDX-License-Identifier: MIT -->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Activity, Shield, Clock, Layers, Server, Sun, Moon,
    Terminal, Lock, FileText, Sliders, Network, LogOut, Copy, Check, Key
  } from 'lucide-svelte';
  import OverviewTab from './lib/OverviewTab.svelte';
  import BackendsTab from './lib/BackendsTab.svelte';
  import ToolsTab from './lib/ToolsTab.svelte';
  import PatTokensTab from './lib/PatTokensTab.svelte';
  import TopologyTab from './lib/TopologyTab.svelte';
  import DlpTab from './lib/DlpTab.svelte';
  import HitlTab from './lib/HitlTab.svelte';
  import PolicyTab from './lib/PolicyTab.svelte';
  import FinopsTab from './lib/FinopsTab.svelte';
  import AuditTab from './lib/AuditTab.svelte';
  import ConfigTab from './lib/ConfigTab.svelte';
  import SearchKeysTab from './lib/SearchKeysTab.svelte';
  import LoginGate from './lib/LoginGate.svelte';

  type Tab = 'overview' | 'backends' | 'tools' | 'search' | 'tokens' | 'topology' | 'policy' | 'config' | 'dlp' | 'hitl' | 'finops' | 'audit';
  type Theme = 'zinc' | 'slate' | 'neutral' | 'oled';

  let currentTab = $state<Tab>('overview');
  let currentTheme = $state<Theme>('zinc');
  let isDark = $state(true);
  let authUser = $state<any>(null);
  let authToken = $state('');
  let copiedToken = $state(false);

  function copyToken() {
    if (!authToken) return;
    navigator.clipboard.writeText(authToken).then(() => {
      copiedToken = true;
      setTimeout(() => { copiedToken = false; }, 2500);
    });
  }

  onMount(() => {
    const params = new URLSearchParams(window.location.search);
    const tokenParam = params.get('token') || (window.location.hash.startsWith('#token=') ? window.location.hash.slice(7) : '');
    if (tokenParam && tokenParam.startsWith('propylon_jwt_')) {
      authToken = tokenParam;
      localStorage.setItem('propylon_saml_token', tokenParam);
      window.history.replaceState({}, document.title, window.location.pathname);
    } else {
      const saved = localStorage.getItem('propylon_saml_token');
      if (saved) authToken = saved;
    }
    if (authToken) loadSession();
  });

  async function loadSession() {
    try {
      const res = await fetch('/api/v1/auth/session', {
        headers: { 'Authorization': `Bearer ${authToken}` }
      });
      if (res.ok) {
        const data = await res.json();
        if (data.authenticated) authUser = data;
      }
    } catch (e) {}
  }

  function handleLogout() {
    authUser = null;
    authToken = '';
    localStorage.removeItem('propylon_saml_token');
    fetch('/api/v1/auth/logout', { method: 'POST' }).catch(() => {});
  }

  function setTheme(theme: Theme) {
    currentTheme = theme;
    document.documentElement.setAttribute('data-theme', theme);
  }

  function toggleMode() {
    isDark = !isDark;
    if (isDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }
</script>

{#if !authUser}
  <LoginGate onManualToken={(tok) => { authToken = tok; localStorage.setItem('propylon_saml_token', tok); loadSession(); }} />
{:else}
<div class="min-h-screen bg-background text-foreground flex flex-col font-sans">
  <!-- Top Global Navigation Bar -->
  <header class="border-b border-border bg-card/70 backdrop-blur sticky top-0 z-50">
    <div class="container mx-auto px-3 sm:px-4 h-14 flex items-center justify-between gap-2">
      <div class="flex items-center gap-2.5 min-w-0">
        <div class="h-7 w-7 shrink-0 rounded bg-primary text-primary-foreground flex items-center justify-center font-bold text-xs tracking-tight shadow-sm">
          PP
        </div>
        <div class="flex items-center gap-2 truncate">
          <span class="font-bold text-sm tracking-tight truncate">PROPYLON GATEWAY</span>
          <span class="hidden sm:inline-block text-[10px] font-medium tracking-wide uppercase text-muted-foreground border border-border px-1.5 py-0.5 rounded">
            CONTROL PLANE
          </span>
        </div>
      </div>

      <!-- Center Status Telemetry -->
      <div class="hidden lg:flex items-center gap-3 text-[11px] font-medium text-muted-foreground">
        <div class="flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></span>
          <span class="text-foreground font-semibold">HEALTH: 100% OK</span>
        </div>
        <span class="text-border">|</span>
        <span class="tabular-nums">LATENCY: 0.38ms (P99)</span>
        <span class="text-border">|</span>
        <span>ZERO-TRUST mTLS</span>
      </div>

      <!-- Center-Right SAML Identity Widget & Token Copy -->
      <div class="flex items-center gap-2">
        <div class="flex items-center gap-2 border border-border bg-muted/40 py-1 px-2.5 rounded-lg text-xs">
          <div class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></div>
          <div class="flex flex-col text-left">
            <span class="font-semibold text-[11px] leading-tight truncate max-w-[110px] sm:max-w-[150px]" title={authUser.subject}>{authUser.subject}</span>
            <span class="text-[9px] text-muted-foreground leading-tight truncate max-w-[110px] sm:max-w-[150px]">{authUser.department || 'BPS Mempawah'} • rIDM SAML</span>
          </div>
          <button
            onclick={() => (currentTab = 'tokens')}
            class="ml-1 text-[10px] font-mono flex items-center gap-1 px-1.5 py-0.5 rounded border border-border bg-background hover:bg-muted text-primary transition-all cursor-pointer shadow-xs font-semibold"
            title="Kelola & Buat Personal Access Token (PAT)"
          >
            <Key class="h-3 w-3" />
            <span>PAT</span>
          </button>
          <button
            onclick={handleLogout}
            class="ml-0.5 text-muted-foreground hover:text-rose-400 text-[10px] flex items-center gap-0.5 transition-colors cursor-pointer"
            title="Logout session"
          >
            <LogOut class="h-3 w-3" />
          </button>
        </div>
      </div>

      <!-- Right Theme Controls -->
      <div class="flex items-center gap-1.5 shrink-0">
        <div class="flex items-center rounded border border-border bg-muted/40 p-0.5 text-xs font-medium">
          {#each (['zinc', 'slate', 'neutral', 'oled'] as const) as th}
            <button
              onclick={() => setTheme(th)}
              class="px-1.5 sm:px-2 py-0.5 rounded text-[10px] sm:text-[11px] font-medium capitalize transition-colors {currentTheme === th ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
            >
              {th}
            </button>
          {/each}
        </div>

        <button
          onclick={toggleMode}
          class="h-7 w-7 sm:h-8 sm:w-8 rounded border border-border bg-card flex items-center justify-center text-muted-foreground hover:text-foreground transition-colors"
          title="Toggle Dark/Light Mode"
        >
          {#if isDark}<Sun class="h-3.5 w-3.5 sm:h-4 sm:w-4" />{:else}<Moon class="h-3.5 w-3.5 sm:h-4 sm:w-4" />{/if}
        </button>
      </div>
    </div>
  </header>

  <!-- Secondary Tab Bar -->
  <nav class="border-b border-border bg-muted/20">
    <div class="container mx-auto px-2 sm:px-4 flex items-center gap-1 overflow-x-auto whitespace-nowrap text-xs font-medium scrollbar-none">
      <button onclick={() => (currentTab = 'overview')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'overview' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Activity class="h-3.5 w-3.5" /> Overview
      </button>
      <button onclick={() => (currentTab = 'tokens')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'tokens' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Key class="h-3.5 w-3.5 text-emerald-400" /> Access Tokens (PAT)
      </button>
      <button onclick={() => (currentTab = 'backends')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'backends' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Server class="h-3.5 w-3.5" /> Backends
      </button>
      <button onclick={() => (currentTab = 'tools')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'tools' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Terminal class="h-3.5 w-3.5" /> Tools
      </button>
      <button onclick={() => (currentTab = 'search')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'search' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Search class="h-3.5 w-3.5 text-blue-400" /> Web Search
      </button>
      <button onclick={() => (currentTab = 'topology')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'topology' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Network class="h-3.5 w-3.5" /> Topology Flow
      </button>
      <button onclick={() => (currentTab = 'policy')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'policy' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Lock class="h-3.5 w-3.5" /> Policy Control
      </button>
      <button onclick={() => (currentTab = 'config')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'config' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Sliders class="h-3.5 w-3.5" /> Config
      </button>
      <button onclick={() => (currentTab = 'dlp')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'dlp' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Shield class="h-3.5 w-3.5" /> DLP Stream
      </button>
      <button onclick={() => (currentTab = 'hitl')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'hitl' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Clock class="h-3.5 w-3.5" /> Suspended HITL
      </button>
      <button onclick={() => (currentTab = 'finops')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'finops' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <Layers class="h-3.5 w-3.5" /> FinOps
      </button>
      <button onclick={() => (currentTab = 'audit')} class="flex items-center gap-2 py-2.5 sm:py-3 px-3 border-b-2 transition-colors {currentTab === 'audit' ? 'border-primary text-primary font-semibold' : 'border-transparent text-muted-foreground hover:text-foreground'}">
        <FileText class="h-3.5 w-3.5" /> SOC 2 Audit
      </button>
    </div>
  </nav>

  <!-- Main Content Viewport -->
  <main class="container mx-auto px-3 sm:px-4 py-4 sm:py-6 flex-1 w-full max-w-full">
    {#if authToken}
      <div class="mb-4 sm:mb-6 p-3 rounded-lg border border-primary/30 bg-primary/5 flex flex-wrap items-center justify-between gap-3 text-xs shadow-xs">
        <div class="flex items-center gap-2.5 min-w-0">
          <Key class="h-4 w-4 text-primary shrink-0" />
          <div class="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 min-w-0">
            <span class="font-semibold text-foreground shrink-0">Kunci Sesi CLI / MCP:</span>
            <code class="px-2 py-0.5 rounded bg-muted/80 font-mono text-[11px] text-primary truncate max-w-[240px] sm:max-w-[400px]">{authToken}</code>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            onclick={copyToken}
            class="px-2.5 py-1 rounded bg-muted hover:bg-muted/80 text-foreground font-medium text-[11px] flex items-center gap-1.5 transition-colors cursor-pointer shadow-xs"
            title="Salin token sesi SAML sementara"
          >
            {#if copiedToken}<Check class="h-3.5 w-3.5 text-emerald-400" /><span class="text-emerald-400">Tersalin!</span>{:else}<Copy class="h-3.5 w-3.5" /><span>Salin Token Sesi</span>{/if}
          </button>
          <button
            onclick={() => (currentTab = 'tokens')}
            class="px-2.5 py-1 rounded bg-primary text-primary-foreground font-medium text-[11px] flex items-center gap-1.5 hover:opacity-90 transition-opacity cursor-pointer shadow-xs"
            title="Buat Personal Access Token tahan 30-90 hari untuk Claude Code / Cursor tanpa install apapun"
          >
            <Key class="h-3.5 w-3.5" />
            <span>Buat PAT (30–90 Hari)</span>
          </button>
        </div>
      </div>
    {/if}

    {#if currentTab === 'overview'}
      <OverviewTab />
    {:else if currentTab === 'tokens'}
      <PatTokensTab />
    {:else if currentTab === 'backends'}
      <BackendsTab />
    {:else if currentTab === 'tools'}
      <ToolsTab />
    {:else if currentTab === 'search'}
      <SearchKeysTab />
    {:else if currentTab === 'topology'}
      <TopologyTab {isDark} />
    {:else if currentTab === 'policy'}
      <PolicyTab />
    {:else if currentTab === 'config'}
      <ConfigTab />
    {:else if currentTab === 'dlp'}
      <DlpTab />
    {:else if currentTab === 'hitl'}
      <HitlTab />
    {:else if currentTab === 'finops'}
      <FinopsTab />
    {:else if currentTab === 'audit'}
      <AuditTab />
    {/if}
  </main>

  <!-- Bottom Technical Footer -->
  <footer class="border-t border-border bg-card/40 py-3 text-[11px] text-muted-foreground">
    <div class="container mx-auto px-3 sm:px-4 flex flex-col sm:flex-row items-center justify-between gap-2 text-center sm:text-left">
      <div class="flex items-center gap-2 flex-wrap justify-center sm:justify-start">
        <Lock class="h-3 w-3 text-emerald-500 shrink-0" />
        <span>Pure-Rust #![deny(unsafe_code)]</span>
        <span class="text-border">|</span>
        <span class="tabular-nums">Zero-Downtime Dynamic Control Plane Active</span>
      </div>
      <div>
        <span>Propylon Enterprise Control Plane v1.34.0</span>
      </div>
    </div>
  </footer>
</div>
{/if}
