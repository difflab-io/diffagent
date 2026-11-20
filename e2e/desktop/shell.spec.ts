import { readFile } from 'node:fs/promises'
import { test, expect } from './fixtures'

test.describe(
  'given a mounted desktop shell over --ipc-bridge',
  { tag: ['@desktop', '@shell'] },
  () => {
    test(
      'startup should render the Helical layout, EmptyState, helix · normal mode badge, and app_info version and mark frontend ready',
      { tag: '@smoke' },
      async ({ page, desktop }) => {
        // Arrange
        const sidebar = page.locator('#navigation-sidebar')
        const mainPane = page.getByRole('main', { name: 'Main content' })
        const statusBar = page.locator('.document-status')
        const modeBadge = statusBar.getByTestId('which-key-mode-badge')

        // Act
        await expect.poll(() => readFile(desktop.ipcLogPath, 'utf8')).toContain('app-ready')
        const ipcLog = await readFile(desktop.ipcLogPath, 'utf8')

        // Assert
        await expect(sidebar).toBeVisible()
        await expect(sidebar).toContainText('No agents configured yet.')
        await expect(mainPane).toContainText('Diffagent')
        await expect(mainPane).toContainText('Press Space for leader bindings')
        await expect(statusBar).toContainText('diffagent-desktop v0.1.0')
        await expect(modeBadge).toHaveText('helix · normal')
        expect(ipcLog).toContain('"command":"app_info"')
        expect(ipcLog).toContain('"command":"app_frontend_ready"')
        expect(ipcLog).toContain(desktop.home)
      },
    )

    test(
      'WhichKey and hotkeys should open the Space leader menu without hardcoded pane slots, toggle the sidebar with Space e and Control+e, switch presets, and open Keybinding Explorer and Jump Mode',
      { tag: '@smoke' },
      async ({ page }) => {
        // Arrange
        const modeBadge = page.locator('.document-status').getByTestId('which-key-mode-badge')
        const popup = page.locator('[data-which-key-root]')
        const mainPane = page.getByRole('main', { name: 'Main content' })
        const before = await mainPane.boundingBox()
        expect(before!.x).toBeGreaterThan(0)

        // Act
        await page.keyboard.press(' ')
        await expect(popup).toBeVisible()
        await expect(popup).toContainText('Toggle Sidebar')
        await expect(popup).not.toContainText('Focus Pane 1')
        const itemKeys = await popup
          .locator('[data-testid="which-key-item"]')
          .evaluateAll((els) => els.map((el) => el.getAttribute('data-key')))
        expect(itemKeys).toEqual([':', '?', 'e', '/', 'j', 'w', 'v', 't', 'k'])

        await page.keyboard.press('e')
        await expect.poll(async () => (await mainPane.boundingBox())?.x).toBe(0)

        await page.keyboard.press('Control+e')
        await expect.poll(async () => (await mainPane.boundingBox())?.x).toBe(before!.x)

        await page.keyboard.press(' ')
        await page.keyboard.press('k')
        await page.keyboard.press('v')
        await expect(modeBadge).toHaveText('vim · normal')

        await page.keyboard.press(' ')
        await page.keyboard.press('k')
        await page.keyboard.press('h')
        await expect(modeBadge).toHaveText('helix · normal')

        await page.keyboard.press(' ')
        await page.keyboard.press('?')
        await expect(page.locator('[data-which-key-explorer]')).toBeVisible()
        await page.keyboard.press('Escape')
        await expect(page.locator('[data-which-key-explorer]')).toHaveCount(0)

        await page.keyboard.press('g')
        await page.keyboard.press('w')

        // Assert
        await expect(modeBadge).toHaveText('helix · jump')
        await expect(page.locator('[data-which-key-jump-overlay]')).toBeVisible()
      },
    )

    test(
      'CommandPalette and CommandMenu should open on : and Mod+k, execute commands, and switch between palette and menu styles',
      { tag: '@smoke' },
      async ({ page }) => {
        // Arrange
        const palette = page.getByTestId('command-palette')
        const paletteInput = page.getByTestId('command-palette-input')
        const menu = page.getByTestId('command-menu')
        const menuInput = page.getByTestId('command-menu-input')
        const mainPane = page.getByRole('main', { name: 'Main content' })
        const initialX = (await mainPane.boundingBox())!.x
        const modShortcut = await page.evaluate(() =>
          /Mac|iPhone|iPad/.test(navigator.platform) ? 'Meta+k' : 'Control+k',
        )

        // Act
        await page.keyboard.press(modShortcut)
        await expect(palette).toBeVisible()
        await paletteInput.fill('Toggle Sidebar')
        await page.keyboard.press('Enter')
        await expect(palette).toHaveCount(0)
        await expect.poll(async () => (await mainPane.boundingBox())?.x).toBe(0)

        await page.keyboard.press(':')
        await expect(palette).toBeVisible()
        await paletteInput.fill('Command Menu')
        await page.keyboard.press('Enter')
        await expect(palette).toHaveCount(0)

        await page.keyboard.press(':')
        await expect(menu).toBeVisible()
        await menuInput.fill('toggle-sidebar')
        await page.keyboard.press('Enter')
        await expect(menu).toHaveCount(0)
        await expect.poll(async () => (await mainPane.boundingBox())?.x).toBe(initialX)

        await page.reload()
        await expect(page.locator('.document-status')).toContainText('diffagent-desktop v0.1.0')
        await expect(mainPane).toBeFocused()
        await page.keyboard.press(':')

        // Assert
        await expect(menu).toBeVisible()
      },
    )
  },
)
