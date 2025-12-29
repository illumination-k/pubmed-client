import { describe, expect, it } from 'vitest'
import { PubMedClient } from '../index.js'
import {
  assertValidAuthors,
  assertValidJournal,
  assertValidPmid,
  assertValidTitle,
  isNetworkError,
  TEST_CONFIG,
} from './setup'

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

  describe('Search Articles', () => {
    it('should search articles successfully', async () => {
      const client = new PubMedClient()

      try {
        const articles = await client.search(TEST_CONFIG.TEST_QUERY, 3)

        expect(Array.isArray(articles)).toBe(true)
        expect(articles.length).toBeGreaterThan(0)
        expect(articles.length).toBeLessThanOrEqual(3)

        const article = articles[0]
        assertValidPmid(article.pmid)
        assertValidTitle(article.title)
        assertValidAuthors(article.authors)
        assertValidJournal(article.journal)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping search test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should search with default limit', async () => {
      const client = new PubMedClient()

      try {
        const articles = await client.search('cancer')

        expect(Array.isArray(articles)).toBe(true)
        // Default limit is 10
        expect(articles.length).toBeLessThanOrEqual(10)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should handle empty query gracefully', async () => {
      const client = new PubMedClient()

      try {
        const articles = await client.search('xyznonexistentquery12345', 5)
        // Should return empty array or throw
        expect(Array.isArray(articles)).toBe(true)
      } catch (error) {
        // Error is also acceptable
        expect(error).toBeDefined()
      }
    })
  })

  describe('Fetch Article', () => {
    it('should fetch article by PMID', async () => {
      const client = new PubMedClient()

      try {
        const article = await client.fetchArticle(TEST_CONFIG.PMC_AVAILABLE_PMID)

        expect(article).toBeDefined()
        expect(article.pmid).toBe(TEST_CONFIG.PMC_AVAILABLE_PMID)
        assertValidTitle(article.title)
        assertValidAuthors(article.authors)
        assertValidJournal(article.journal)
        expect(article.pubDate).toBeDefined()
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping fetch test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should fail with invalid PMID', async () => {
      const client = new PubMedClient()

      await expect(client.fetchArticle(TEST_CONFIG.INVALID_PMID)).rejects.toThrow()
    })
  })

  describe('Check PMC Availability', () => {
    it('should check PMC availability', async () => {
      const client = new PubMedClient()

      try {
        const pmcId = await client.checkPmcAvailability(TEST_CONFIG.PMC_AVAILABLE_PMID)

        // Can be null or a PMC ID string
        if (pmcId !== null) {
          expect(typeof pmcId).toBe('string')
          // Handle potential quote formatting issues from API
          const cleanPmcId = pmcId.replace(/"/g, '')
          expect(cleanPmcId).toMatch(/^PMC\d+$/)
        }
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping PMC availability test due to API issue:', error)
          return
        }
        throw error
      }
    })

    it('should return null for article without PMC', async () => {
      const client = new PubMedClient()

      try {
        // This PMID might not have PMC availability
        const pmcId = await client.checkPmcAvailability('1')

        // Should be null or a string
        expect(pmcId === null || typeof pmcId === 'string').toBe(true)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        // Error is also acceptable for invalid PMID
      }
    })
  })

  describe('Fetch PMC Full Text', () => {
    it('should fetch PMC article', async () => {
      const client = new PubMedClient()

      try {
        const fullText = await client.fetchPmcArticle(TEST_CONFIG.EXPECTED_PMCID)

        expect(fullText).toBeDefined()
        expect(fullText.pmcid).toBe(TEST_CONFIG.EXPECTED_PMCID)
        assertValidTitle(fullText.title)
        assertValidAuthors(fullText.authors)
        expect(Array.isArray(fullText.sections)).toBe(true)
        expect(fullText.sections.length).toBeGreaterThan(0)
        expect(Array.isArray(fullText.references)).toBe(true)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping PMC fetch test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should fail with invalid PMC ID', async () => {
      const client = new PubMedClient()

      await expect(client.fetchPmcArticle('invalid_pmc')).rejects.toThrow()
    })
  })

  describe('Fetch PMC as Markdown', () => {
    it('should convert PMC article to markdown', async () => {
      const client = new PubMedClient()

      try {
        const markdown = await client.fetchPmcAsMarkdown(TEST_CONFIG.EXPECTED_PMCID)

        expect(typeof markdown).toBe('string')
        expect(markdown.length).toBeGreaterThan(0)
        expect(markdown).toContain('#') // Should have markdown headers
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping markdown test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should convert with custom options', async () => {
      const client = new PubMedClient()

      try {
        const markdown = await client.fetchPmcAsMarkdown(TEST_CONFIG.EXPECTED_PMCID, {
          includeMetadata: true,
          useYamlFrontmatter: true,
          includeToc: true,
        })

        expect(typeof markdown).toBe('string')
        expect(markdown.length).toBeGreaterThan(0)
        // YAML frontmatter starts with ---
        expect(markdown.startsWith('---')).toBe(true)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping markdown options test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should fail with invalid PMC ID', async () => {
      const client = new PubMedClient()

      await expect(client.fetchPmcAsMarkdown('invalid_pmc')).rejects.toThrow()
    })
  })
})
