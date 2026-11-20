/**
 * @module-tag e2e
 * @module-tag cli
 */
import { chmod, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { useTestHarness } from '../common/fixtures.js'
import { waitFor } from '../common/test-utils.js'

describe('when running diffagent open', { tags: ['e2e', 'cli', 'open'] }, () => {
  const fixture = useTestHarness('open')

  it('should exit 1 with not_found given a missing desktop binary', async () => {
    // Arrange
    const { runtime } = fixture()
    const missingDesktop = join(runtime.directory, 'missing-diffagent-desktop')

    // Act
    const result = await runtime.cli(['open', '--json'], {
      DIFFAGENT_DESKTOP_BIN: missingDesktop,
    })

    // Assert
    expect(result.code).toBe(1)
    expect(JSON.parse(result.stdout)).toEqual({
      error: {
        code: 'not_found',
        message: `Desktop executable not found: ${missingDesktop}`,
      },
    })
  })

  it('should exit nonzero given unexpected arguments', async () => {
    // Arrange
    const { runtime } = fixture()

    // Act
    const result = await runtime.cli(['open', './unexpected-path', '--json'])

    // Assert
    expect(result.code).not.toBe(0)
    expect(JSON.parse(result.stdout).error.code).toBe('invalid_argument')
  })

  it(
    'should spawn diffagent-desktop detached and return the executable path given a valid desktop binary',
    { tags: ['smoke'] },
    async () => {
      // Arrange
      const { runtime } = fixture()
      const markerPath = join(runtime.directory, 'desktop-launched.log')
      const fakeDesktop = join(runtime.directory, 'fake-diffagent-desktop')
      await writeFile(
        fakeDesktop,
        `#!/bin/sh\nprintf '%s|%s\\n' "$#" "$DIFFAGENT_HOME" >> "${markerPath}"\n`,
        'utf8',
      )
      await chmod(fakeDesktop, 0o755)

      // Act
      const output = await runtime.json(['open'], {
        DIFFAGENT_DESKTOP_BIN: fakeDesktop,
      })

      // Assert
      expect(output).toEqual({ executable: fakeDesktop })
      const launched = await waitFor(
        'detached desktop marker file',
        () => readFile(markerPath, 'utf8'),
        (content) => content.trim().length > 0,
      )
      expect(launched.trim()).toBe(`0|${runtime.home}`)
    },
  )
})
