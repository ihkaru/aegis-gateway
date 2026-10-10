<script lang="ts">
  import {
    Activity,
    Shield,
    Clock,
    Layers,
    Server,
    Sun,
    Moon,
    Terminal,
    Lock,
    Cpu,
  } from 'lucide-svelte';
  import OverviewTab from './lib/OverviewTab.svelte';
  import DlpTab from './lib/DlpTab.svelte';
  import HitlTab from './lib/HitlTab.svelte';
  import FinopsTab from './lib/FinopsTab.svelte';

  type Tab = 'overview' | 'dlp' | 'hitl' | 'finops';
  type Theme = 'zinc' | 'slate' | 'neutral' | 'oled';

  let currentTab = $state<Tab>('overview');
  let currentTheme = $state<Theme>('zinc');
  let isDark = $state(true);

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

<div class="min-h-screen bg-background text-foreground flex flex-col font-sans">
  <!-- Top Global Navigation Bar -->
  <header class="border-b border-border bg-card/60 backdrop-blur sticky top-0 z-50">
    <div class="container mx-auto px-4 h-14 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="h-7 w-7 rounded bg-primary text-primary-foreground flex items-center justify-center font-mono font-bold text-xs tracking-tighter">
          AG
        </div>
        <div>
          <span class="font-bold text-sm tracking-tight">AEGIS GATEWAY</span>
          <span class="ml-2 text-[10px] font-mono uppercase text-muted-foreground border border-border px-1.5 py-0.5 rounded">
            CONTROL PLANE
          </span>
        </div>
      </div>

      <!-- Center Status Telemetry -->
      <div class="hidden lg:flex items-center gap-3 text-[11px] font-mono text-muted-foreground">
        <div class="flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></span>
          <span class="text-foreground font-medium">HEALTH: 100% OK</span>
        </div>
        <span class="text-border">|</span>
        <span>LATENCY: 0.38ms (P99)</span>
        <span class="text-border">|</span>
        <span>SECURITY: ZERO-TRUST mTLS</span>
      </div>

      <!-- Right Theme Controls -->
      <div class="flex items-center gap-2">
        <div class="flex items-center rounded border border-border bg-muted/40 p-0.5 text-xs font-mono">
          <button
            onclick={() => setTheme('zinc')}
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {currentTheme === 'zinc' ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
          >
            Zinc
          </button>
          <button
            onclick={() => setTheme('slate')}
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {currentTheme === 'slate' ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
          >
            Slate
          </button>
          <button
            onclick={() => setTheme('neutral')}
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {currentTheme === 'neutral' ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
          >
            Neutral
          </button>
          <button
            onclick={() => setTheme('oled')}
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {currentTheme === 'oled' ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
          >
            OLED
          </button>
        </div>

        <button
          onclick={toggleMode}
          class="h-8 w-8 rounded border border-border bg-card flex items-center justify-center text-muted-foreground hover:text-foreground transition-colors"
          title="Toggle Dark/Light Mode"
        >
          {#if isDark}
            <Sun class="h-4 w-4" />
          {:else}
            <Moon class="h-4 w-4" />
          {/if}
        </button>
      </div>
    </div>
  </header>

  <!-- Secondary Tab Bar -->
  <nav class="border-b border-border bg-muted/20">
    <div class="container mx-auto px-4 flex items-center gap-1 overflow-x-auto text-xs font-medium">
      <button
        onclick={() => (currentTab = 'overview')}
        class="flex items-center gap-2 py-3 px-3.5 border-b-2 font-mono transition-colors {currentTab === 'overview' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'}"
      >
        <Activity class="h-3.5 w-3.5" />
        Overview & Metrics
      </button>
      <button
        onclick={() => (currentTab = 'dlp')}
        class="flex items-center gap-2 py-3 px-3.5 border-b-2 font-mono transition-colors {currentTab === 'dlp' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'}"
      >
        <Shield class="h-3.5 w-3.5" />
        DLP Security Stream
      </button>
      <button
        onclick={() => (currentTab = 'hitl')}
        class="flex items-center gap-2 py-3 px-3.5 border-b-2 font-mono transition-colors {currentTab === 'hitl' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'}"
      >
        <Clock class="h-3.5 w-3.5" />
        Suspended HITL Tasks
      </button>
      <button
        onclick={() => (currentTab = 'finops')}
        class="flex items-center gap-2 py-3 px-3.5 border-b-2 font-mono transition-colors {currentTab === 'finops' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'}"
      >
        <Layers class="h-3.5 w-3.5" />
        Tenant FinOps Quotas
      </button>
    </div>
  </nav>

  <!-- Main Content Viewport -->
  <main class="container mx-auto px-4 py-6 flex-1">
    {#if currentTab === 'overview'}
      <OverviewTab />
    {:else if currentTab === 'dlp'}
      <DlpTab />
    {:else if currentTab === 'hitl'}
      <HitlTab />
    {:else if currentTab === 'finops'}
      <FinopsTab />
    {/if}
  </main>

  <!-- Bottom Technical Footer -->
  <footer class="border-t border-border bg-card/40 py-3 text-[11px] font-mono text-muted-foreground">
    <div class="container mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-2">
      <div class="flex items-center gap-2">
        <Lock class="h-3 w-3 text-emerald-500" />
        <span>Pure-Rust #![deny(unsafe_code)]</span>
        <span class="text-border">|</span>
        <span>Memory Consumption: 24.2 MB RSS</span>
      </div>
      <div>
        <span>Aegis Enterprise Control Plane v1.0.0 (Phase 33 Standard)</span>
      </div>
    </div>
  </footer>
</div>
