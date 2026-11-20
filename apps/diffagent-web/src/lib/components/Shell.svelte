<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import { getDocumentWhichKeyController, type CommandStyle } from '@cloudvoyant/helical-ui'
  import {
    CommandMenu,
    CommandPalette,
    CommandProvider,
    Pane,
    Row,
    Sidebar,
    SidebarContent,
    SidebarHeader,
    SidebarProvider,
    SidebarTrigger,
    WhichKey,
    WhichKeyProvider,
  } from '@cloudvoyant/helical-svelte'
  import { EmptyState } from 'diffagent-ui'
  import PanelLeft from 'lucide-svelte/icons/panel-left'
  import {
    desktopClient,
    errorMessage,
    type AppInfo,
    type DesktopClient,
  } from '../clients/desktopClient'
  import { HotkeyService } from '../services/hotkeyService.svelte'
  import StatusBar from './StatusBar.svelte'

  // Props ---------------------------------------------------------------------
  interface ShellProps {
    client?: DesktopClient
  }
  let { client = desktopClient }: ShellProps = $props()

  // State ---------------------------------------------------------------------
  const hotkeys = untrack(() => new HotkeyService())
  let appInfo = $state<AppInfo | null>(null)
  let startupError = $state<string | null>(null)

  // Lifecycle -----------------------------------------------------------------
  onMount(() => {
    let active = true

    void (async () => {
      try {
        const info = await client.getAppInfo()
        if (!active) return
        appInfo = info
        await client.markFrontendReady()
        await tick()
        requestAnimationFrame(() => {
          if (active && (!document.activeElement || document.activeElement === document.body)) {
            getDocumentWhichKeyController(document)?.focusPane('main')
          }
        })
      } catch (err) {
        if (active) startupError = errorMessage(err)
      }
    })()

    return () => {
      active = false
    }
  })

  // Methods -------------------------------------------------------------------
  function handleSidebarOpenChange(next: boolean): void {
    hotkeys.sidebarOpen = next
  }

  function handleCommandStyleChange(next: CommandStyle): void {
    hotkeys.setCommandStyle(next)
  }
</script>

<svelte:head><title>Diffagent</title></svelte:head>

<WhichKeyProvider
  preset="helix"
  storageKey="diffagent"
  scoped={false}
  keymap={hotkeys.keymap}
  commands={hotkeys.commands}
>
  <CommandProvider
    bind:open={hotkeys.commandOpen}
    items={hotkeys.commandItems}
    style={hotkeys.commandStyle}
    onStyleChange={handleCommandStyleChange}
  >
    <SidebarProvider
      open={hotkeys.sidebarOpen}
      onOpenChange={handleSidebarOpenChange}
      class="app-shell flex h-dvh min-h-0 flex-col gap-0 overflow-hidden"
      style="--sidebar-width: 236px"
    >
      <header
        class="titlebar flex h-9 shrink-0 items-center gap-3 border-b border-border bg-sidebar pr-4 pl-20 text-sidebar-foreground select-none"
        data-tauri-drag-region
      >
        <SidebarTrigger title="Toggle sidebar" aria-controls="navigation-sidebar">
          <PanelLeft size={14} strokeWidth={1.5} />
        </SidebarTrigger>
        <span class="text-xs font-semibold tracking-tight" data-tauri-drag-region>Diffagent</span>
      </header>

      <Row class="min-h-0 flex-1 items-stretch gap-0">
        <Sidebar
          id="navigation-sidebar"
          paneId="sidebar"
          paneTitle="Navigation"
          collapsible="offcanvas"
          class="navigation-sidebar top-9 bottom-8.5 h-auto w-59 shrink-0 border-r border-sidebar-border bg-sidebar outline-none"
          aria-label="Navigation"
        >
          <SidebarHeader
            class="sticky top-0 z-1 shrink-0 bg-sidebar px-3.5 pt-3.5 pb-2.5 text-[11px] font-semibold text-muted-foreground"
          >
            <span>Navigation</span>
          </SidebarHeader>
          <SidebarContent>
            <p class="mx-3.5 my-2 text-xs leading-relaxed text-muted-foreground">
              No agents configured yet.
            </p>
          </SidebarContent>
        </Sidebar>

        <Pane
          paneId="main"
          paneTitle="Main"
          paneOrder={2}
          role="main"
          tabindex={-1}
          aria-label="Main content"
          class="main-pane relative flex min-h-0 min-w-0 flex-1 flex-col bg-background text-foreground outline-none"
        >
          <EmptyState
            title="Diffagent"
            description="Press Space for leader bindings or : and Mod+K for the command surface."
          />
        </Pane>
      </Row>

      <StatusBar {appInfo} error={startupError} />

      <CommandPalette />
      <CommandMenu hotkey="Mod+k" />
    </SidebarProvider>
  </CommandProvider>

  <WhichKey />
</WhichKeyProvider>
