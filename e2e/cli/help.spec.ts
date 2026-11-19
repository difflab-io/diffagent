/**
 * @module-tag e2e
 * @module-tag cli
 */
import { readdir } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'
import { useTestHarness } from '../common/fixtures.js'

describe(
  'when running help, version, and status commands',
  { tags: ['e2e', 'cli', 'help'] },
  () => {
    const fixture = useTestHarness('help')

    it(
      'bare diffagent, --help, and --version should exit 0, list status and open, and print the CLI version without mutating diffagent-home/',
      { tags: ['smoke'] },
      async () => {
        // Arrange
        const { runtime } = fixture()

        // Act
        const bare = await runtime.cli([])
        const help = await runtime.cli(['--help'])
        const version = await runtime.cli(['--version'])
        const homeEntries = await readdir(runtime.home)

        // Assert
        expect(bare.code).toBe(0)
        expect(help.code).toBe(0)
        expect(version.code).toBe(0)
        for (const subcommand of ['status', 'open']) {
          expect(bare.stdout).toContain(subcommand)
          expect(help.stdout).toContain(subcommand)
        }
        expect(version.stdout.trim()).toMatch(/^diffagent \d+\.\d+\.\d+$/)
        expect(homeEntries).toEqual([])
      },
    )

    it(
      'status --json should return version and home_dir without mutating diffagent-home/',
      { tags: ['smoke'] },
      async () => {
        // Arrange
        const { runtime } = fixture()

        // Act
        const payload = await runtime.json(['status'])
        const homeEntries = await readdir(runtime.home)

        // Assert
        expect(payload).toEqual({
          version: '0.1.0',
          home_dir: runtime.home,
        })
        expect(homeEntries).toEqual([])
      },
    )

    it('should exit nonzero without mutating diffagent-home/ given an unknown subcommand', async () => {
      // Arrange
      const { runtime } = fixture()

      // Act
      const unknown = await runtime.cli(['unknown-subcommand', '--json'])
      const homeEntries = await readdir(runtime.home)

      // Assert
      expect(unknown.code).not.toBe(0)
      expect(JSON.parse(unknown.stdout).error.code).toBe('invalid_argument')
      expect(homeEntries).toEqual([])
    })
  },
)
