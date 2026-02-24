import { describe, expect, it } from 'vitest'
import { PubMedClient } from '../index.js'
import { TEST_CONFIG } from './setup'

describe('PubMedClient', () => {
  describe('Client Creation', () => {
    it('should create client with default config', () => {
      const client = new PubMedClient()
      expect(client).toBeDefined()
      expect(typeof client.search).toBe('function')
      expect(typeof client.fetchArticle).toBe('function')
      expect(typeof client.fetchPmcArticle).toBe('function')
      expect(typeof client.fetchPmcAsMarkdown).toBe('function')
      expect(typeof client.checkPmcAvailability).toBe('function')
    })

    it('should create client with custom config', () => {
      const client = PubMedClient.withConfig({
        email: TEST_CONFIG.TEST_EMAIL,
        tool: TEST_CONFIG.TEST_TOOL,
        timeoutSeconds: 30,
      })
      expect(client).toBeDefined()
    })

    it('should create client with API key', () => {
      const client = PubMedClient.withConfig({
        apiKey: 'test-api-key',
        email: TEST_CONFIG.TEST_EMAIL,
      })
      expect(client).toBeDefined()
    })
  })
})
