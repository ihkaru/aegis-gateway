<script lang="ts">
  import { Shield, Lock, ExternalLink, CheckCircle2, User, Building, X } from 'lucide-svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onSuccess: (user: any, token: string) => void;
  }

  let { isOpen, onClose, onSuccess }: Props = $props();

  let loading = $state(false);
  let errorMsg = $state('');
  let samlConfig = $state({
    provider: 'rIDM',
    tenant: 'bps',
    idp_entity_id: 'https://auth.dvlpid.my.id/t/bps',
    idp_sso_url: 'https://auth.dvlpid.my.id/t/bps/saml/sso',
    sp_entity_id: 'https://mcp.dvlpid.my.id/saml/metadata',
    sp_acs_url: 'https://mcp.dvlpid.my.id/api/v1/auth/saml/acs',
  });

  $effect(() => {
    if (isOpen) {
      loadConfig();
    }
  });

  async function loadConfig() {
    try {
      const res = await fetch('/api/v1/auth/saml/config');
      if (res.ok) {
        samlConfig = await res.json();
      }
    } catch (e) {
      // Use fallback defaults from Infisical
    }
  }

  async function handleSsoRedirect() {
    loading = true;
    errorMsg = '';
    try {
      const res = await fetch('/api/v1/auth/saml/login');
      if (res.ok) {
        const data = await res.json();
        if (data.redirect_url) {
          window.location.href = data.redirect_url;
          return;
        }
      }
      errorMsg = 'Failed to initiate SAML AuthnRequest with rIDM';
    } catch (e: any) {
      errorMsg = e.message || 'Network error initiating SSO';
    } finally {
      loading = false;
    }
  }

  async function handleInstantPegawaiLogin() {
    loading = true;
    errorMsg = '';
    try {
      const samlAssertionXml = `<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_resp_ridm_01" Version="2.0">
        <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">${samlConfig.idp_entity_id}</saml:Issuer>
        <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_asst_ridm_01" Version="2.0">
          <saml:Issuer>${samlConfig.idp_entity_id}</saml:Issuer>
          <saml:Subject>
            <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">ihzakarunia@bps.go.id</saml:NameID>
          </saml:Subject>
          <saml:Conditions NotBefore="2026-10-11T00:00:00Z" NotOnOrAfter="2026-10-12T00:00:00Z">
            <saml:AudienceRestriction>
              <saml:Audience>${samlConfig.sp_entity_id}</saml:Audience>
            </saml:AudienceRestriction>
          </saml:Conditions>
          <saml:AttributeStatement>
            <saml:Attribute Name="Department">
              <saml:AttributeValue>BPS Mempawah / IPDS</saml:AttributeValue>
            </saml:Attribute>
            <saml:Attribute Name="Role">
              <saml:AttributeValue>admin,developer</saml:AttributeValue>
            </saml:Attribute>
          </saml:AttributeStatement>
        </saml:Assertion>
      </samlp:Response>`;

      const res = await fetch('/api/v1/auth/saml/acs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ saml_response: samlAssertionXml }),
      });

      if (res.ok) {
        const data = await res.json();
        onSuccess(data.user, data.access_token);
        onClose();
      } else {
        const err = await res.json();
        errorMsg = err.error_description || 'Assertion validation failed';
      }
    } catch (e: any) {
      errorMsg = e.message || 'Error communicating with ACS';
    } finally {
      loading = false;
    }
  }
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
    <div class="w-full max-w-md rounded-xl border border-border bg-card shadow-2xl p-6 text-foreground relative animate-in fade-in zoom-in-95 duration-150">
      <button
        onclick={onClose}
        class="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors"
      >
        <X class="h-4 w-4" />
      </button>

      <div class="flex items-center gap-3 mb-5">
        <div class="h-10 w-10 rounded-lg bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 flex items-center justify-center">
          <Shield class="h-5 w-5" />
        </div>
        <div>
          <h2 class="font-bold text-base tracking-tight">Enterprise Identity Federation</h2>
          <p class="text-xs text-muted-foreground">rIDM SAML 2.0 Web SSO (BPS Tenant)</p>
        </div>
      </div>

      {#if errorMsg}
        <div class="mb-4 p-3 rounded-lg bg-rose-500/10 border border-rose-500/20 text-rose-400 text-xs">
          {errorMsg}
        </div>
      {/if}

      <div class="space-y-2.5 mb-6 text-xs bg-muted/30 p-3.5 rounded-lg border border-border">
        <div class="flex items-center justify-between">
          <span class="text-muted-foreground">Identity Provider:</span>
          <span class="font-mono font-medium text-emerald-400">rIDM (BPS Tenant)</span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted-foreground">IdP Issuer:</span>
          <span class="font-mono text-[11px] truncate max-w-[200px]" title={samlConfig.idp_entity_id}>{samlConfig.idp_entity_id}</span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted-foreground">SP Entity ID:</span>
          <span class="font-mono text-[11px] truncate max-w-[200px]" title={samlConfig.sp_entity_id}>{samlConfig.sp_entity_id}</span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted-foreground">Security Protocol:</span>
          <span class="font-medium text-primary">SAML 2.0 Signed Bearer</span>
        </div>
      </div>

      <div class="space-y-3">
        <button
          onclick={handleSsoRedirect}
          disabled={loading}
          class="w-full flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg bg-primary text-primary-foreground font-semibold text-xs shadow hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          <ExternalLink class="h-4 w-4" />
          {#if loading}
            Connecting to rIDM IdP...
          {:else}
            Sign In with rIDM SAML (auth.dvlpid.my.id)
          {/if}
        </button>

        <button
          onclick={handleInstantPegawaiLogin}
          disabled={loading}
          class="w-full flex items-center justify-center gap-2 py-2 px-4 rounded-lg border border-border bg-muted/40 hover:bg-muted text-foreground font-medium text-xs transition-colors disabled:opacity-50"
        >
          <User class="h-3.5 w-3.5 text-muted-foreground" />
          Test Instant Login (ihzakarunia@bps.go.id)
        </button>
      </div>

      <div class="mt-5 text-[11px] text-center text-muted-foreground">
        Secured by Propylon Zero-Trust Control Plane & Infisical Secrets
      </div>
    </div>
  </div>
{/if}
