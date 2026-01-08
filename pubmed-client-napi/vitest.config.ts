import { resolve } from 'node:path'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    setupFiles: ['./tests/setup.ts'],
    include: ['tests/**/*.test.ts'],
    testTimeout: 30000, // 30 seconds for network operations
    hookTimeout: 10000,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['index.js'],
      exclude: ['index.d.ts', '*.node', 'node_modules/', 'tests/', 'examples/'],
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
      // Help Vitest resolve ./index.js when imported from streaming.js
      './index.js': resolve(__dirname, './index.js'),
    },
  },
  esbuild: {
    target: 'node18',
  },
})
