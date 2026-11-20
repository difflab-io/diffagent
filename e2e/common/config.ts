import { defineConfig } from 'vitest/config'

/** Builds a sequential Vitest configuration with standard E2E tags. */
export function suiteConfig(
  root: string,
  options: { include?: string[]; exclude?: string[] } = {},
) {
  return defineConfig({
    test: {
      root,
      tags: [
        { name: 'extended', skip: process.env.DIFFAGENT_E2E_EXTENDED !== 'true' },
        ...['e2e', 'smoke', 'cli', 'help', 'open'].map((name) => ({ name })),
      ],
      include: options.include ?? [process.env.DIFFAGENT_E2E_SPEC ?? '**/*.spec.ts'],
      exclude: ['**/.cache/**', ...(options.exclude ?? [])],
      testTimeout: 60_000,
      hookTimeout: 30_000,
      sequence: { hooks: 'stack' },
      fileParallelism: false,
      maxWorkers: 1,
      reporters: ['verbose'],
      retry: 0,
    },
  })
}
