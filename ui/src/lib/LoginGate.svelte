<!-- SPDX-License-Identifier: MIT -->
<script lang="ts">
  import { Shield, Lock, ExternalLink, KeyRound, Building, ArrowRight } from 'lucide-svelte';

  interface Props {
    onManualToken: (token: string) => void;
  }

  let { onManualToken }: Props = $props();

  let manualTokenInput = $state('');
  let showManualInput = $state(false);

  function handleSsoRedirect() {
    window.location.href = '/saml/login';
  }

  function handleManualSubmit() {
    if (manualTokenInput.trim()) {
      onManualToken(manualTokenInput.trim());
    }
  }
</script>

<div class="min-h-screen bg-slate-950 flex flex-col justify-center items-center p-6 text-slate-100">
  <div class="w-full max-w-md bg-slate-900 border border-slate-800 rounded-2xl p-8 shadow-2xl relative overflow-hidden">
    <!-- Glowing background accent -->
    <div class="absolute -top-24 -right-24 w-48 h-48 bg-emerald-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute -bottom-24 -left-24 w-48 h-48 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none"></div>

    <div class="flex items-center space-x-3 mb-6">
      <div class="p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-xl text-emerald-400">
        <Shield class="w-7 h-7" />
      </div>
      <div>
        <h1 class="text-xl font-bold tracking-tight text-white">Propylon Gateway</h1>
        <p class="text-xs text-slate-400 font-mono">Strict Zero-Trust Access Barrier</p>
      </div>
    </div>

    <div class="bg-slate-950/70 border border-slate-800/80 rounded-xl p-4 mb-6 text-xs space-y-2">
      <div class="flex items-center justify-between">
        <span class="text-slate-400 flex items-center gap-1.5"><Building class="w-3.5 h-3.5" /> Identity Provider</span>
        <span class="font-semibold text-emerald-400">rIDM BPS (SAML 2.0)</span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-slate-400 flex items-center gap-1.5"><Lock class="w-3.5 h-3.5" /> Policy Enforced</span>
        <span class="font-mono text-cyan-400">Strict Airgapped</span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-slate-400">Tenant Domain</span>
        <span class="font-mono text-slate-300">auth.dvlpid.my.id/t/bps</span>
      </div>
    </div>

    <p class="text-xs text-slate-400 leading-relaxed mb-6">
      Akses ke MCP Control Plane, Sandbox Tools, dan Topology Engine diproteksi oleh protokol Zero-Trust. Autentikasi akun korporat Anda melalui rIDM untuk melanjutkan.
    </p>

    <button
      onclick={handleSsoRedirect}
      class="w-full py-3 px-4 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 text-white font-medium rounded-xl transition duration-150 flex items-center justify-center space-x-2 shadow-lg shadow-emerald-950 cursor-pointer"
    >
      <span>Masuk via rIDM SSO BPS</span>
      <ExternalLink class="w-4 h-4" />
    </button>

    <div class="mt-6 pt-6 border-t border-slate-800/60">
      {#if !showManualInput}
        <button
          onclick={() => showManualInput = true}
          class="w-full text-center text-xs text-slate-400 hover:text-slate-200 transition flex items-center justify-center gap-1.5 cursor-pointer"
        >
          <KeyRound class="w-3.5 h-3.5" />
          <span>Gunakan Token CLI / Kunci Akses Manual</span>
        </button>
      {:else}
        <div class="space-y-3">
          <label for="manual-token-input" class="text-[11px] text-slate-400 font-mono block">Masukkan Token JWT (`propylon_jwt_...`):</label>
          <div class="flex gap-2">
            <input
              id="manual-token-input"
              type="password"
              bind:value={manualTokenInput}
              placeholder="propylon_jwt_..."
              class="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500 font-mono"
            />
            <button
              onclick={handleManualSubmit}
              class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-white text-xs rounded-lg font-medium transition cursor-pointer flex items-center gap-1"
            >
              <span>Verifikasi</span>
              <ArrowRight class="w-3 h-3" />
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
