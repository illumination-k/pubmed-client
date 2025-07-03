import { resolve } from 'node:path'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    setupFiles: ['./tests/setup.ts'],
    include: ['tests/**/*.test.ts'],
    exclude: ['tests/test_wasm_integration.js'],
    testTimeout: 30000, // 30 seconds for network operations
    hookTimeout: 10000,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['pkg/**/*.js'],
      exclude: ['pkg/**/*.d.ts', 'pkg/**/*.wasm', 'node_modules/', 'tests/'],
    },
    pool: 'threads',
    poolOptions: {
      threads: {
        singleThread: false,
        maxThreads: 4,
        minThreads: 1,
      },
    },
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './tests'),
      '@/tests': resolve(__dirname, './tests'),
      '@/pkg': resolve(__dirname, './pkg'),
    },
  },
  esbuild: {
    target: 'node18',
  },
})
