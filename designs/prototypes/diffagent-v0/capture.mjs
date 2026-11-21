import assert from 'node:assert/strict'
import { mkdir } from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, join } from 'node:path'

// Optional external install keeps this browser-only prototype dependency-free.
const playwrightPkg = process.env.DIFFAGENT_PLAYWRIGHT_MODULE || '@playwright/test'
const { chromium } = await import(playwrightPkg)

const root = dirname(fileURLToPath(import.meta.url))
const browser = await chromium.launch({
  headless: true,
  ...(process.env.DIFFAGENT_BROWSER_CHANNEL
    ? { channel: process.env.DIFFAGENT_BROWSER_CHANNEL }
    : {}),
})

const page = await browser.newPage({
  viewport: { width: 1120, height: 640 },
  deviceScaleFactor: 1,
})

const errors = []
page.on('pageerror', (error) => errors.push(error.message))

const base = pathToFileURL(join(root, 'index.html')).href

async function open(hash) {
  await page.goto(`${base}#${hash}`, { waitUntil: 'networkidle' })
  await page.reload({ waitUntil: 'networkidle' })
  await page.waitForFunction(
    () => typeof DOMPurify !== 'undefined' && typeof HSStaticMethods !== 'undefined',
  )
}

try {
  // Verify top bar layout (logo left, centered tabs, right Settings drawer), no footer, no new chat.
  await open('chat')
  assert.equal(await page.locator('.statusbar').count(), 0)
  assert.equal(await page.locator('#bot-filter').count(), 0)
  assert.equal(await page.getByRole('button', { name: 'New chat' }).count(), 0)
  await page.locator('[data-select-bot="pr-concierge"]').click()
  assert.equal(await page.locator('.top-title').textContent(), 'PR Concierge')
  assert.ok((await page.locator('.topbar').textContent()).includes('Remote · prod-us'))

  // Verify inline approval gate, sending a message, and jumping to trace waterfall.
  await open('chat-approval')
  await page.getByRole('button', { name: 'Allow once', exact: true }).click()
  await page.locator('.toast').filter({ hasText: 'Allowed once' }).waitFor()
  await page.getByLabel('Message bot').fill('Check status of DIF-118.')
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  assert.ok((await page.locator('.chat-stream').textContent()).includes('Check status of DIF-118.'))
  await page.getByRole('button', { name: 'Inspect trace →' }).first().click()
  assert.ok((await page.locator('.filter-bar').first().textContent()).includes('tr_891a'))

  // Verify minimal New Bot creation (name + avatar shape only, automatic directory, no connectors).
  await open('chat')
  await page.getByRole('button', { name: 'New bot', exact: true }).click()
  assert.equal(await page.locator('#nb-path').count(), 0)
  await page.getByLabel('Shape hex').click()
  await page.getByLabel('Name').fill('Ops Responder')
  await page.getByRole('button', { name: 'Create bot', exact: true }).click()
  assert.equal(await page.locator('.top-title').textContent(), 'Ops Responder')

  // Verify right-hand Bot Settings sidebar (Skills, Connectors, Files).
  await open('chat')
  await page.getByRole('button', { name: 'Settings', exact: true }).click()
  assert.equal(await page.locator('#bot-settings-drawer').count(), 1)
  await page.getByRole('tab', { name: 'Skills' }).click()
  await page.getByRole('button', { name: '＋ Add skill' }).click()
  await page.getByRole('tab', { name: 'Files' }).click()
  assert.ok(
    (await page.locator('#bot-settings-drawer').textContent()).includes('custom-skill-4/SKILL.md'),
  )

  // Verify Statistics, Sessions, and Trace Waterfall interactions.
  await open('stats')
  await page.getByRole('button', { name: '24h', exact: true }).click()
  assert.ok((await page.locator('.kpi-grid').textContent()).includes('186'))

  await open('sessions')
  await page.getByLabel('Search sessions').fill('Discord')
  assert.equal(await page.locator('[data-select-session]').count(), 1)
  await page.locator('[data-select-session="sess_dsc_44f1"]').click()
  assert.ok((await page.locator('.split-right').textContent()).includes('dev_kieran'))

  await open('trace-detail')
  await page.getByRole('button', { name: 'Tree', exact: true }).click()
  await page.getByRole('button', { name: 'Metadata', exact: true }).click()
  assert.ok((await page.locator('.split-right').textContent()).includes('anthropic'))

  // Verify square-tile Connectors hub + custom MCP + bottom-left user avatar menu.
  await open('user-connectors')
  assert.ok((await page.locator('.conn-tile').count()) >= 8)
  await page.getByRole('button', { name: 'Add custom MCP' }).click()
  await page.getByRole('button', { name: 'Add MCP', exact: true }).click()
  assert.ok((await page.locator('.tile-grid').textContent()).includes('PagerDuty MCP'))

  await open('chat')
  await page.getByRole('button', { name: 'Account menu' }).click()
  assert.ok((await page.locator('.account-menu').textContent()).includes('Log out'))

  await mkdir(join(root, 'snapshots'), { recursive: true })

  const screens = [
    ['empty', 'empty'],
    ['bot-chat', 'chat'],
    ['bot-chat-remote', 'chat-remote'],
    ['bot-chat-approval', 'chat-approval'],
    ['bot-create', 'new-bot'],
    ['bot-stats', 'stats'],
    ['bot-sessions', 'sessions'],
    ['bot-traces', 'traces'],
    ['bot-trace-waterfall', 'trace-detail'],
    ['bot-settings-general', 'bot-settings'],
    ['bot-settings-skills', 'bot-skills'],
    ['bot-settings-connectors', 'bot-connectors'],
    ['bot-settings-channels', 'bot-channels'],
    ['bot-settings-files', 'bot-files'],
    ['user-connectors', 'user-connectors'],
    ['user-connectors-mcp', 'add-mcp'],
    ['user-account-menu', 'user-menu'],
    ['user-settings', 'user-settings'],
  ]

  for (const [name, hash] of screens) {
    await open(hash)
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth),
      1120,
      `Expected no horizontal overflow on #${hash}`,
    )
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollHeight),
      640,
      `Expected exact viewport fit on #${hash}`,
    )
    await page.screenshot({ path: join(root, 'snapshots', `${name}.png`) })
  }

  // Verify narrow viewport fits without horizontal overflow.
  await page.setViewportSize({ width: 768, height: 720 })
  await open('chat')
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 768)

  assert.deepEqual(errors, [])
  console.log(`Interaction checks passed. Captured ${screens.length} snapshots at 1120 x 640.`)
} finally {
  await browser.close()
}
