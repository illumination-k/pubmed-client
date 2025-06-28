import { defineConfig } from 'vitest/config'
import { resolve } from 'path'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    setupFiles: ['./tests/wasm/setup.ts'],
    include: ['tests/wasm/**/*.test.ts'],
    exclude: ['tests/wasm/test_wasm_integration.js'],
    testTimeout: 30000, // 30 seconds for network operations
    hookTimeout: 10000,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['pkg/**/*.js'],
      exclude: [
        'pkg/**/*.d.ts',
        'pkg/**/*.wasm',
        'node_modules/',
        'tests/',
      ],
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
      '@/wasm': resolve(__dirname, './tests/wasm'),
      '@/pkg': resolve(__dirname, './pkg'),
    },
  },
  esbuild: {
    target: 'node18',
  },
})
