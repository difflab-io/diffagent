<script lang="ts">
  import { WhichKeyMode } from '@cloudvoyant/helical-svelte'
  import type { AppInfo } from '../clients/desktopClient'

  // Props ---------------------------------------------------------------------
  interface StatusBarProps {
    appInfo: AppInfo | null
    error: string | null
  }
  let { appInfo, error }: StatusBarProps = $props()
</script>

<footer
  class="document-status relative z-20 flex shrink-0 items-center justify-between gap-4 border-t border-border bg-sidebar px-4 py-2 text-[11px] text-muted-foreground"
  role="status"
  aria-live="polite"
>
  <span class="truncate">
    {#if error}
      <span class="text-destructive">{error}</span>
    {:else if appInfo}
      {appInfo.name} v{appInfo.version}
    {:else}
      Starting diffagent…
    {/if}
  </span>
  <div class="flex shrink-0 items-center gap-3">
    <WhichKeyMode showPreset aria-label="Keymap mode" />
  </div>
</footer>
