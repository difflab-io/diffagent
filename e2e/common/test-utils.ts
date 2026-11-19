import { homedir } from 'node:os'
import { join, resolve, sep } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Json = Record<string, any>

/** Converts a test title into a filesystem-safe directory slug. */
export function slugify(label: string): string {
  return (
    label
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 96) || 'case'
  )
}

/** Rejects test home paths that resolve to the developer's real `~` or `~/.diffagent` directory. */
export function assertIsolatedTestHome(targetHome: string): void {
  const resolvedTarget = resolve(targetHome)
  const realUserHome = resolve(homedir())
  const realDiffagentHome = resolve(realUserHome, '.diffagent')

  if (
    resolvedTarget === realUserHome ||
    resolvedTarget === realDiffagentHome ||
    resolvedTarget.startsWith(`${realDiffagentHome}${sep}`)
  ) {
    throw new Error(`Refusing to run test against non-isolated DIFFAGENT_HOME: ${resolvedTarget}`)
  }
}

/** Polls `read()` until `ready(value)` succeeds or `timeout` elapses. */
export async function waitFor<T>(
  description: string,
  read: () => Promise<T>,
  ready: (value: T) => boolean = Boolean,
  timeout = 10_000,
): Promise<T> {
  const deadline = Date.now() + timeout
  let last: unknown
  while (Date.now() < deadline) {
    try {
      const value = await read()
      if (ready(value)) return value
    } catch (error) {
      last = error
    }
    await delay(25)
  }
  throw new Error(`Timed out waiting for ${description}`, { cause: last })
}

/** Builds an isolated child-process environment rooted at `homeDir`. */
export function isolatedEnv(homeDir: string): NodeJS.ProcessEnv {
  assertIsolatedTestHome(homeDir)
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    HOME: homeDir,
    DIFFAGENT_HOME: homeDir,
    GIT_CEILING_DIRECTORIES: homeDir,
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_CONFIG_NOSYSTEM: '1',
    XDG_CONFIG_HOME: join(homeDir, 'config'),
    XDG_DATA_HOME: join(homeDir, 'data'),
    XDG_STATE_HOME: join(homeDir, 'state'),
    XDG_CACHE_HOME: join(homeDir, 'cache'),
  }
  delete env.DIFFAGENT_DESKTOP_BIN
  return env
}
