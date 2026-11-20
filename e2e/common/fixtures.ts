import { afterEach, beforeEach } from 'vitest'
import { TestHarness } from './test-harness.js'

/** Provisions a deterministic `e2e/outputs/cli/<spec>/<slug>/` context per test case. */
export function useTestHarness(spec: string, environment?: () => NodeJS.ProcessEnv) {
  let runtime: TestHarness | undefined

  beforeEach(async ({ task }) => {
    runtime = await TestHarness.create(spec, task.name, environment?.())
  })

  afterEach(() => {
    runtime = undefined
  })

  return () => {
    if (!runtime) throw new Error('TestHarness requested outside an active test')
    return { runtime }
  }
}
