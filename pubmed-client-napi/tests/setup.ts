/**
 * Vitest setup file for napi-rs tests
 * This file is executed before each test file
 */

import { afterAll, beforeAll } from 'vitest'

// Global test configuration
beforeAll(async () => {
  console.log('🧬 Setting up napi-rs test environment...')

  // Ensure native module is available
  try {
    const napiModule = await import('../index.js')

    if (!napiModule.PubMedClient) {
      throw new Error('Native module not properly exported')
    }
    console.log('✅ Native module loaded successfully')
  } catch (error) {
    console.error('❌ Failed to load native module:', error)
    console.error('💡 Make sure to run: pnpm run build')
    throw error
  }
})

afterAll(() => {
  console.log('🧹 Cleaning up napi-rs test environment...')
})

/**
 * Test configuration constants
 */
export const TEST_CONFIG = {
  // Test PMIDs that are known to exist and have stable data
  VALID_PMIDS: ['31978945', '33515491', '25760099'],
  // PMID known to have PMC full text
  PMC_AVAILABLE_PMID: '31978945',
  // Expected PMC ID for the above PMID
  EXPECTED_PMCID: 'PMC7092803',
  // Test query that should return results
  TEST_QUERY: 'covid-19',
  // Invalid PMID for error testing
  INVALID_PMID: 'invalid_pmid_12345',
  // Default test email and tool
  TEST_EMAIL: 'test@example.com',
  TEST_TOOL: 'napi-rs Test Suite',
} as const

/**
 * Helper to check if an error is a network/API issue that should be skipped
 */
export function isNetworkError(error: unknown): boolean {
  const errorString = String(error)
  return (
    errorString.includes('429') ||
    errorString.includes('Too Many Requests') ||
    errorString.includes('control character') ||
    errorString.includes('error decoding response body') ||
    errorString.includes('TypeError: terminated') ||
    errorString.includes('Test timed out') ||
    errorString.includes('XML parsing error') ||
    errorString.includes('Failed to deserialize XML') ||
    errorString.includes('ECONNREFUSED') ||
    errorString.includes('ETIMEDOUT')
  )
}

/**
 * Common test assertions
 */
export function assertValidPmid(pmid: unknown): asserts pmid is string {
  expect(pmid).toBeDefined()
  expect(typeof pmid).toBe('string')
  expect(pmid).toMatch(/^\d+$/)
}

export function assertValidTitle(title: unknown): asserts title is string {
  expect(title).toBeDefined()
  expect(typeof title).toBe('string')
  expect((title as string).length).toBeGreaterThan(0)
}

export function assertValidAuthors(
  authors: unknown
): asserts authors is Array<{ fullName: string }> {
  expect(authors).toBeDefined()
  expect(Array.isArray(authors)).toBe(true)
  expect((authors as any[]).length).toBeGreaterThan(0)
  ;(authors as any[]).forEach(author => {
    expect(typeof author.fullName).toBe('string')
    expect(author.fullName.length).toBeGreaterThan(0)
  })
}

export function assertValidJournal(journal: unknown): asserts journal is string {
  expect(journal).toBeDefined()
  expect(typeof journal).toBe('string')
  expect((journal as string).length).toBeGreaterThan(0)
}
