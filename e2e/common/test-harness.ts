import { spawn } from 'node:child_process'
import { once } from 'node:events'
import { appendFile, mkdir, rm, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { expect } from 'vitest'
import { assertIsolatedTestHome, isolatedEnv, slugify, type Json } from './test-utils.js'

export const ROOT = resolve(import.meta.dirname, '../..')
export const OUTPUTS_ROOT = join(ROOT, 'e2e/outputs')

/** Resolves a compiled Cargo workspace debug binary by name. */
export function binary(name: string): string {
  const exe = process.platform === 'win32' ? `${name}.exe` : name
  return join(process.env.CARGO_TARGET_DIR ?? join(ROOT, 'target'), 'debug', exe)
}

function formatCliArg(arg: string): string {
  return /[\s"']/.test(arg) ? JSON.stringify(arg) : arg
}

/** Spawns a subprocess, captures stdout and stderr, and enforces a timeout. */
export async function command(
  program: string,
  args: string[],
  options: { cwd?: string; env?: NodeJS.ProcessEnv; input?: string; timeout?: number } = {},
) {
  const child = spawn(program, args, {
    cwd: options.cwd ?? ROOT,
    env: options.env,
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  let stdout = ''
  let stderr = ''
  child.stdout.on('data', (chunk) => {
    stdout += chunk
  })
  child.stderr.on('data', (chunk) => {
    stderr += chunk
  })
  child.stdin.end(options.input)
  const timer = setTimeout(() => child.kill('SIGKILL'), options.timeout ?? 15_000)
  try {
    const [code] = await once(child, 'close')
    return { code: code as number | null, stdout, stderr }
  } finally {
    clearTimeout(timer)
  }
}

/** Owns an isolated `e2e/outputs/cli/<spec>/<slug>/` directory and records `cli.log` transcripts. */
export class TestHarness {
  readonly env: NodeJS.ProcessEnv

  private constructor(
    readonly directory: string,
    readonly home: string,
    readonly cliLogPath: string,
  ) {
    this.env = isolatedEnv(home)
  }

  /** Creates a fresh deterministic output directory for one CLI test case. */
  static async create(spec: string, testName: string, extraEnv: NodeJS.ProcessEnv = {}) {
    const slug = slugify(testName)
    const directory = join(OUTPUTS_ROOT, 'cli', spec, slug)
    const home = join(directory, 'diffagent-home')
    const cliLogPath = join(directory, 'cli.log')

    assertIsolatedTestHome(home)
    await rm(directory, { recursive: true, force: true })
    await mkdir(home, { recursive: true })
    await writeFile(cliLogPath, `# Spec: e2e/cli/${spec}.spec.ts\n# Test: ${testName}\n`, 'utf8')

    const harness = new TestHarness(directory, home, cliLogPath)
    Object.assign(harness.env, extraEnv)
    return harness
  }

  /** Executes `diffagent` with `args` and appends the full transcript to `cli.log`. */
  async cli(args: string[], envOverrides: NodeJS.ProcessEnv = {}) {
    const result = await command(binary('diffagent'), args, {
      cwd: this.directory,
      env: { ...this.env, ...envOverrides },
      timeout: 30_000,
    })
    const sections = [
      '',
      '================================================================================',
      `$ diffagent ${args.map(formatCliArg).join(' ')}`.trimEnd(),
      `[exit ${result.code ?? 'null'}]`,
    ]
    if (result.stdout.trimEnd()) {
      sections.push(result.stdout.trimEnd())
    }
    if (result.stderr.trimEnd()) {
      sections.push('--- stderr ---', result.stderr.trimEnd())
    }
    await appendFile(this.cliLogPath, `${sections.join('\n')}\n`, 'utf8')
    return result
  }

  /** Executes `diffagent ... --json`, asserts exit code `0`, and parses JSON stdout. */
  async json(args: string[], envOverrides: NodeJS.ProcessEnv = {}): Promise<Json> {
    const response = await this.cli([...args, '--json'], envOverrides)
    expect(response.code, `${args.join(' ')}\n${response.stderr}\n${response.stdout}`).toBe(0)
    return JSON.parse(response.stdout) as Json
  }
}
