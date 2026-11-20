import { test as base, expect, type Page } from '@playwright/test'
import { spawn } from 'node:child_process'
import { appendFile, mkdir, rm, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'
import { assertIsolatedTestHome, isolatedEnv, slugify } from '../common/test-utils'

export { expect }

/** Isolated desktop bridge context retained under `e2e/outputs/desktop/shell/<slug>/`. */
export interface DesktopContext {
  directory: string
  home: string
  ipcLogPath: string
  emit: (event: string, payload?: unknown) => Promise<void>
}

interface BridgeMessage {
  id?: number
  ok?: unknown
  error?: { message?: string } | string
  event?: string
  payload?: unknown
}

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const outputsRoot = join(repoRoot, 'e2e', 'outputs')
const desktopBinary = join(
  repoRoot,
  'apps',
  'diffagent-desktop',
  'target',
  'debug',
  process.platform === 'win32' ? 'diffagent-desktop.exe' : 'diffagent-desktop',
)

async function emit(page: Page, event: string, payload?: unknown): Promise<void> {
  if (page.isClosed()) return
  await page
    .evaluate(
      async ({ name, data }) => {
        await (
          window as unknown as {
            desktopTestEmit: (event: string, payload?: unknown) => Promise<void>
          }
        ).desktopTestEmit(name, data)
      },
      { name: event, data: payload },
    )
    .catch(() => {})
}

// Fixtures --------------------------------------------------------------------
// The browser mounts the production `Shell.svelte` component and routes every
// Tauri IPC command through the real `diffagent-desktop --ipc-bridge` Rust binary.
export const test = base.extend<{
  desktop: DesktopContext
}>({
  desktop: [
    async ({ page }, use, testInfo) => {
      const slug = slugify(`${testInfo.project.name}-${testInfo.title}`)
      const directory = join(outputsRoot, 'desktop', 'shell', slug)
      const home = join(directory, 'diffagent-home')
      const ipcLogPath = join(directory, 'ipc.log')

      assertIsolatedTestHome(home)
      await rm(directory, { recursive: true, force: true })
      await mkdir(home, { recursive: true })
      await writeFile(
        ipcLogPath,
        `# Spec: e2e/desktop/shell.spec.ts\n# Project: ${testInfo.project.name}\n# Test: ${testInfo.title}\n`,
        'utf8',
      )

      let logChain: Promise<void> = Promise.resolve()
      function appendIpcLog(line: string): Promise<void> {
        logChain = logChain.then(() => appendFile(ipcLogPath, `${line}\n`, 'utf8'))
        return logChain
      }

      const bridge = spawn(desktopBinary, ['--ipc-bridge', home], {
        env: {
          ...isolatedEnv(home),
          DIFFAGENT_IPC_BRIDGE: '1',
        },
        stdio: ['pipe', 'pipe', 'inherit'],
      })

      let nextId = 1
      const pending = new Map<
        number,
        { resolve: (value: unknown) => void; reject: (reason: Error) => void }
      >()

      const rl = createInterface({ input: bridge.stdout })
      rl.on('line', (line) => {
        const trimmed = line.trim()
        if (!trimmed) return
        const logged = appendIpcLog(`<- ${trimmed}`)
        let msg: BridgeMessage
        try {
          msg = JSON.parse(trimmed) as BridgeMessage
        } catch {
          return
        }
        if (msg.event) {
          const eventName = msg.event
          const payload = msg.payload
          void logged.then(() => emit(page, eventName, payload))
          return
        }
        if (typeof msg.id === 'number') {
          const waiter = pending.get(msg.id)
          if (!waiter) return
          pending.delete(msg.id)
          if ('error' in msg && msg.error !== undefined) {
            const errMsg =
              typeof msg.error === 'object' && msg.error && 'message' in msg.error
                ? String(msg.error.message)
                : String(msg.error)
            void logged.then(() => waiter.reject(new Error(errMsg)))
          } else {
            const ok = msg.ok
            void logged.then(() => waiter.resolve(ok))
          }
        }
      })

      async function invokeBridge(
        command: string,
        args: Record<string, unknown>,
      ): Promise<unknown> {
        const id = nextId++
        const frame = JSON.stringify({ id, command, args })
        await appendIpcLog(`-> ${frame}`)
        return new Promise((resolvePromise, rejectPromise) => {
          pending.set(id, { resolve: resolvePromise, reject: rejectPromise })
          bridge.stdin.write(`${frame}\n`)
        })
      }

      const context: DesktopContext = {
        directory,
        home,
        ipcLogPath,
        emit: (eventName, payload) => emit(page, eventName, payload),
      }

      try {
        await page.exposeFunction(
          'desktopTestInvoke',
          async (command: string, args: Record<string, unknown> = {}) => {
            if (command === 'plugin:window|destroy') {
              return null
            }
            return await invokeBridge(command, args)
          },
        )
        await page.goto('/e2e/desktop/fixture.html')
        await expect(page.locator('.document-status')).toContainText('diffagent-desktop v0.1.0')
        await expect(page.getByRole('main', { name: 'Main content' })).toBeFocused()
        await use(context)
      } finally {
        try {
          if (testInfo.status !== testInfo.expectedStatus && !page.isClosed()) {
            await testInfo.attach('failure', {
              body: await page.screenshot({ timeout: 2000 }),
              contentType: 'image/png',
            })
          }
        } finally {
          try {
            await page.close()
          } finally {
            rl.close()
            bridge.stdin.end()
            bridge.kill()
            await logChain.catch(() => {})
          }
        }
      }
    },
    { auto: true },
  ],
})
