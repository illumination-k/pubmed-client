/**
 * Vitest setup file for WASM tests
 * This file is executed before each test file
 */

import { afterAll, afterEach, beforeAll, beforeEach } from 'vitest'

// Global test configuration
beforeAll(async () => {
  console.log('🧬 Setting up WASM test environment...')

  // Ensure WASM module is available and initialized
  try {
    // Import the WASM module (Node.js target auto-initializes)
    const wasmModule = await import('../pkg/pubmed_client_wasm.js')

    if (!wasmModule.WasmPubMedClient || !wasmModule.WasmClientConfig) {
      throw new Error('WASM module not properly exported')
    }
    console.log('✅ WASM module loaded successfully')
  } catch (error) {
    console.error('❌ Failed to load WASM module:', error)
    console.error('💡 Make sure to run: npm run build')
    throw error
  }
})

afterAll(() => {
  console.log('🧹 Cleaning up WASM test environment...')
})

// Per-test setup
let createdClients: Array<{ free?: () => void }> = []
let createdConfigs: Array<{ free?: () => void }> = []

beforeEach(() => {
  // Reset arrays for tracking created objects
  createdClients = []
  createdConfigs = []
})

afterEach(() => {
  // Clean up any WASM objects that were created during the test
  createdClients.forEach(client => {
    try {
      if (client.free) {
        client.free()
      }
    } catch (error) {
      // Ignore cleanup errors
      console.warn('Warning: Failed to free client:', error)
    }
  })

  createdConfigs.forEach(config => {
    try {
      if (config.free) {
        config.free()
      }
    } catch (error) {
      // Ignore cleanup errors
      console.warn('Warning: Failed to free config:', error)
    }
  })

  createdClients = []
  createdConfigs = []
})

/**
 * Helper to track WASM objects for automatic cleanup
 */
export function trackClient<T extends { free?: () => void }>(client: T): T {
  createdClients.push(client)
  return client
}

export function trackConfig<T extends { free?: () => void }>(config: T): T {
  createdConfigs.push(config)
  return config
}

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
  TEST_TOOL: 'WASM Test Suite',
  // Rate limits for testing
  DEFAULT_RATE_LIMIT: 3.0,
  TEST_RATE_LIMIT: 5.0,
  // Timeouts
  DEFAULT_TIMEOUT_SECONDS: 30n,
  QUICK_TIMEOUT_SECONDS: 5n,
} as const

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
  expect(title.length).toBeGreaterThan(0)
}

export function assertValidAuthors(authors: unknown): asserts authors is string[] {
  expect(authors).toBeDefined()
  expect(Array.isArray(authors)).toBe(true)
  expect(authors.length).toBeGreaterThan(0)
  authors.forEach(author => {
    expect(typeof author).toBe('string')
    expect(author.length).toBeGreaterThan(0)
  })
}

export function assertValidJournal(journal: unknown): asserts journal is string {
  expect(journal).toBeDefined()
  expect(typeof journal).toBe('string')
  expect(journal.length).toBeGreaterThan(0)
}

export function assertValidDate(date: unknown): asserts date is string {
  expect(date).toBeDefined()
  expect(typeof date).toBe('string')
  expect(date.length).toBeGreaterThan(0)
}

/**
 * Mock data for testing without network calls
 */
export const MOCK_ARTICLE = {
  pmid: '31978945',
  title: 'A Novel Coronavirus from Patients with Pneumonia in China, 2019',
  authors: ['Zhu N', 'Zhang D', 'Wang W'],
  journal: 'New England Journal of Medicine',
  pub_date: '2020 Feb 20',
  doi: '10.1056/NEJMoa2001017',
  article_types: ['Journal Article'],
  abstract_text: 'In December 2019, a cluster of patients with pneumonia...',
}

export const MOCK_FULL_TEXT = {
  pmcid: 'PMC7092803',
  pmid: '31978945',
  title: 'A Novel Coronavirus from Patients with Pneumonia in China, 2019',
  authors: [
    {
      given_names: 'Na',
      surname: 'Zhu',
      full_name: 'Na Zhu',
      affiliations: ['State Key Laboratory of Virology'],
      is_corresponding: false,
    },
  ],
  journal: {
    title: 'New England Journal of Medicine',
    abbreviation: 'N Engl J Med',
    issn_print: '0028-4793',
  },
  pub_date: '2020 Feb 20',
  doi: '10.1056/NEJMoa2001017',
  sections: [
    {
      section_type: 'abstract',
      title: 'Abstract',
      content: 'In December 2019, a cluster of patients with pneumonia...',
    },
  ],
  references: [],
  keywords: ['coronavirus', 'pneumonia', 'outbreak'],
}
