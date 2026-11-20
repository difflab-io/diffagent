import { tick } from 'svelte'
import {
  getDocumentWhichKeyController,
  type CommandNode,
  type CommandStyle,
  type KeybindingPreset,
  type KeymapMode,
  type WhichKeyNode,
} from '@cloudvoyant/helical-ui'

const COMMAND_STYLE_STORAGE_KEY = 'diffagent.commandStyle'

function leaf(
  key: string,
  label: string,
  category: string,
  commandId: string,
  description: string,
  mode?: KeymapMode[],
): WhichKeyNode {
  return { key, label, category, commandId, description, ...(mode ? { mode } : {}) }
}

function loadSavedCommandStyle(): CommandStyle {
  if (typeof localStorage === 'undefined') return 'palette'
  const saved = localStorage.getItem(COMMAND_STYLE_STORAGE_KEY)
  return saved === 'menu' ? 'menu' : 'palette'
}

/** Shell keyboard shortcuts, leader menu, command catalog, and spatial pane navigation. */
export class HotkeyService {
  /** Whether the active `CommandPalette` or `CommandMenu` overlay is open. */
  commandOpen = $state(false)

  /** Active command surface style (`palette` for `⌘K` modal, `menu` for `:` prompt). */
  commandStyle = $state<CommandStyle>(loadSavedCommandStyle())

  /** Whether the left navigation sidebar is expanded. */
  sidebarOpen = $state(true)

  /** Root `WhichKey` keymap defining `:`, `Control+e`, and `<leader>` (`Space`) chords. */
  readonly keymap: Record<string, WhichKeyNode> = {
    ':': leaf(
      ':',
      'Open Command',
      'Leader',
      'command.open',
      'Open the active Command Palette or Command Menu',
      ['normal', 'select'],
    ),
    'Control+e': leaf(
      'Control+e',
      'Toggle Sidebar',
      'Navigation',
      'shell.toggleSidebar',
      'Toggle the navigation sidebar open or closed',
      ['normal', 'select'],
    ),
    Space: {
      key: 'Space',
      label: '+leader',
      group: 'Leader',
      category: 'Leader',
      mode: ['normal', 'select'],
      expand: () => {
        const viewKey = this.controller?.getState().preset === 'kakoune' ? 'v' : 'z'
        return {
          ':': leaf(
            ':',
            'Open Command',
            'Leader',
            'command.open',
            'Open the active Command Palette or Command Menu (<leader>:)',
          ),
          '?': leaf(
            '?',
            'Keybind Help',
            'Leader',
            'whichKey.openExplorer',
            'Open searchable keybinding exploration menu (<leader>?)',
          ),
          e: leaf(
            'e',
            'Toggle Sidebar',
            'Navigation',
            'shell.toggleSidebar',
            'Toggle the navigation sidebar open or closed',
          ),
          '/': leaf(
            '/',
            'Search',
            'Leader',
            'whichKey.openExplorer',
            'Search commands and keybindings',
          ),
          j: leaf(
            'j',
            'Jump to Element',
            'Navigation',
            'jump.start',
            'Show 2D home-row jump labels over visible controls',
          ),
          w: {
            key: 'w',
            label: '+window',
            group: 'Window / Pane',
            description: 'Spatial pane navigation',
            category: 'Window / Pane',
            proxy: ['Control+w'],
          },
          v: {
            key: 'v',
            label: '+view',
            group: 'View / Scroll',
            description: 'Scroll and align viewport',
            category: 'View',
            proxy: [viewKey],
          },
          t: {
            key: 't',
            label: '+toggles',
            group: 'Toggles',
            category: 'Preferences',
            children: {
              w: leaf(
                'w',
                'Toggle WhichKey Leader Only',
                'Preferences',
                'settings.toggleWhichKeyLeaderOnly',
                'Toggle whether the WhichKey popup opens for all prefixes or only <leader>',
              ),
            },
          },
          k: {
            key: 'k',
            label: '+preset',
            group: 'Keybinding Preset',
            description: 'Switch modal grammar (Helix / Vim / Kakoune)',
            category: 'Leader',
            children: {
              h: leaf(
                'h',
                'Helix Preset',
                'Preset',
                'preset.helix',
                'Switch to Helix selection-first grammar (default)',
              ),
              v: leaf(
                'v',
                'Vim Preset',
                'Preset',
                'preset.vim',
                'Switch to Vim operator-motion grammar',
              ),
              k: leaf(
                'k',
                'Kakoune Preset',
                'Preset',
                'preset.kakoune',
                'Switch to Kakoune object-verb grammar',
              ),
            },
          },
        }
      },
    },
  }

  /** Command handlers invoked by `WhichKeyProvider` when a bound chord fires. */
  readonly commands: Record<string, () => boolean> = {
    'command.open': () => this.openCommand(),
    'shell.toggleSidebar': () => this.toggleSidebar(),
    'settings.toggleWhichKeyLeaderOnly': () => {
      const ctrl = this.controller
      if (!ctrl) return false
      ctrl.setLeaderOnly(!ctrl.getState().leaderOnly)
      return true
    },
  }

  /** Opens the active command surface (`CommandPalette` or `CommandMenu`). */
  openCommand(): boolean {
    this.commandOpen = true
    return true
  }

  /** Updates the open state of the active command surface. */
  setCommandOpen(open: boolean): void {
    this.commandOpen = open
  }

  /** Persists and applies the selected command surface style (`palette` or `menu`). */
  setCommandStyle(next: CommandStyle): void {
    this.commandStyle = next
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(COMMAND_STYLE_STORAGE_KEY, next)
    }
  }

  /** Toggles the navigation sidebar and transfers spatial focus to the target pane. */
  toggleSidebar(): boolean {
    const nextOpen = !this.sidebarOpen
    this.sidebarOpen = nextOpen
    const targetPane = nextOpen ? 'sidebar' : 'main'
    void tick().then(() => {
      this.controller?.focusPane(targetPane)
    })
    return true
  }

  /** Focuses the navigation sidebar pane, expanding it first if collapsed. */
  focusSidebarPane(): void {
    if (!this.sidebarOpen) {
      this.sidebarOpen = true
      void tick().then(() => {
        this.controller?.focusPane('sidebar')
      })
      return
    }
    this.controller?.focusPane('sidebar')
  }

  /** Focuses the main content pane. */
  focusMainPane(): void {
    this.controller?.focusPane('main')
  }

  /** Shared command catalog consumed by both `CommandPalette` and `CommandMenu`. */
  get commandItems(): CommandNode[] {
    const wkState = this.controller?.getState()
    const currentPreset: KeybindingPreset = wkState?.preset ?? 'helix'
    const currentLeaderOnly = Boolean(wkState?.leaderOnly)

    return [
      {
        value: 'toggle-sidebar',
        label: 'Toggle Sidebar',
        description: 'Toggle the navigation sidebar open or closed',
        group: 'Navigation',
        command: 'toggle-sidebar',
        aliases: ['sidebar', 'explorer'],
        shortcut: 'Space e',
        keywords: ['sidebar', 'panel', 'navigation', 'toggle'],
        onSelect: () => {
          this.toggleSidebar()
        },
      },
      {
        value: 'focus-sidebar',
        label: 'Focus Sidebar Pane',
        description: 'Move keyboard focus to the navigation sidebar',
        group: 'Navigation',
        command: 'focus-sidebar',
        aliases: ['pane-sidebar'],
        keywords: ['focus', 'sidebar', 'pane'],
        onSelect: () => {
          this.focusSidebarPane()
        },
      },
      {
        value: 'focus-main',
        label: 'Focus Main Pane',
        description: 'Move keyboard focus to the main content pane',
        group: 'Navigation',
        command: 'focus-main',
        aliases: ['pane-main'],
        keywords: ['focus', 'main', 'pane'],
        onSelect: () => {
          this.focusMainPane()
        },
      },
      {
        value: 'jump',
        label: 'Jump to Element',
        description: 'Overlay 2D home-row jump badges across visible controls',
        group: 'Navigation',
        command: 'jump',
        aliases: ['gw'],
        shortcut: 'Space j',
        keywords: ['jump', 'flash', 'hints', 'labels'],
        onSelect: () => {
          this.controller?.startJumpMode()
        },
      },
      {
        value: 'command-style',
        label: 'Change Command Style',
        description: 'Choose between Command Palette (⌘K) and Command Menu (:)',
        group: 'Preferences',
        command: 'command-style',
        aliases: ['cmd', 'command'],
        keywords: ['palette', 'menu', 'colon', 'prompt', 'modal'],
        children: [
          {
            value: 'command-style:palette',
            label: 'Command Palette (⌘K)',
            description: 'Centered fuzzy-searchable command modal',
            command: 'palette',
            active: this.commandStyle === 'palette',
            onSelect: () => {
              this.setCommandStyle('palette')
            },
          },
          {
            value: 'command-style:menu',
            label: 'Command Menu (:)',
            description: 'Bottom Helix/Kakoune-style :command prompt with completions',
            command: 'menu',
            aliases: ['colon'],
            active: this.commandStyle === 'menu',
            onSelect: () => {
              this.setCommandStyle('menu')
            },
          },
        ],
      },
      {
        value: 'which-key-activation',
        label: 'WhichKey Activation',
        description: 'Configure when the WhichKey chord helper popup appears',
        group: 'Preferences',
        command: 'which-key-activation',
        aliases: ['which-key', 'wk', 'leader-only'],
        shortcut: 'Space t w',
        keywords: ['leader', 'popup', 'prefix', 'chords', 'whichkey', 'activation'],
        children: [
          {
            value: 'which-key-activation:all',
            label: 'All Prefixes',
            description: 'Show WhichKey popup for all multi-key prefixes (g, z, Ctrl+w, Space)',
            command: 'all',
            aliases: ['full', 'every'],
            active: !currentLeaderOnly,
            onSelect: () => {
              this.controller?.setLeaderOnly(false)
            },
          },
          {
            value: 'which-key-activation:leader',
            label: 'Leader Only',
            description: 'Show WhichKey popup only when pressing <leader> (Space)',
            command: 'leader',
            aliases: ['space', 'minimal'],
            active: currentLeaderOnly,
            onSelect: () => {
              this.controller?.setLeaderOnly(true)
            },
          },
        ],
      },
      {
        value: 'preset',
        label: 'Change Keybinding Preset',
        description: 'Switch modal editing grammar (Helix, Vim, or Kakoune)',
        group: 'Preferences',
        command: 'preset',
        aliases: ['keybind', 'keymap', 'k'],
        shortcut: 'Space k',
        keywords: ['helix', 'vim', 'kakoune', 'modal', 'grammar'],
        children: [
          {
            value: 'preset:helix',
            label: 'Helix',
            description: 'Selection-first modal grammar (default)',
            command: 'helix',
            aliases: ['hx'],
            active: currentPreset === 'helix',
            onSelect: () => {
              this.controller?.setPreset('helix')
            },
          },
          {
            value: 'preset:vim',
            label: 'Vim',
            description: 'Operator-motion modal grammar',
            command: 'vim',
            active: currentPreset === 'vim',
            onSelect: () => {
              this.controller?.setPreset('vim')
            },
          },
          {
            value: 'preset:kakoune',
            label: 'Kakoune',
            description: 'Object-verb modal grammar',
            command: 'kakoune',
            aliases: ['kak'],
            active: currentPreset === 'kakoune',
            onSelect: () => {
              this.controller?.setPreset('kakoune')
            },
          },
        ],
      },
      {
        value: 'keybindings',
        label: 'Explore Keybindings',
        description: 'Open the searchable keybinding reference dialog',
        group: 'Tools',
        command: 'keybindings',
        aliases: ['bindings', 'keys', 'help'],
        shortcut: 'Space ?',
        keywords: ['shortcuts', 'chords', 'which-key', 'reference'],
        onSelect: () => {
          this.controller?.openExplorer()
        },
      },
    ]
  }

  private get controller() {
    return typeof document !== 'undefined' ? getDocumentWhichKeyController(document) : null
  }
}
