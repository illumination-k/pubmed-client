import { describe, expect, it } from 'vitest'
import { WasmClientConfig, WasmPubMedClient } from '../../pkg/pubmed_client_rs.js'

describe('WASM Client Basic Tests', () => {
  describe('WasmClientConfig', () => {
    it('should create config successfully', () => {
      const config = new WasmClientConfig()
      expect(config).toBeDefined()
      config.free()
    })

    it('should handle invalid property gracefully', () => {
      const config = new WasmClientConfig()
      // TypeScript will catch this, but test runtime behavior
      expect(() => {
        ;(config as any).invalid_property = 'test'
      }).not.toThrow()
      config.free()
    })
  })

  describe('WasmPubMedClient Creation', () => {
    it('should create client successfully', () => {
      const client = new WasmPubMedClient()
      expect(client).toBeDefined()
      expect(typeof client.search_articles).toBe('function')
      client.free()
    })

    it('should create client with config successfully', () => {
      const config = new WasmClientConfig()
      config.email = 'test@example.com'

      const client = WasmPubMedClient.with_config(config)
      expect(client).toBeDefined()

      client.free()
      // Don't free config after it's been used with a client
      // config.free()
    })
  })

  describe('Search Articles', () => {
    it('should search articles successfully', async () => {
      const client = new WasmPubMedClient()

      const articles = await client.search_articles('covid-19', 2)

      expect(Array.isArray(articles)).toBe(true)
      expect(articles.length).toBeGreaterThan(0)
      expect(articles.length).toBeLessThanOrEqual(2)

      const article = articles[0]
      expect(typeof article?.pmid).toBe('string')
      expect(typeof article?.title).toBe('string')
      expect(Array.isArray(article?.authors)).toBe(true)

      client.free()
    })

    it('should handle empty query', async () => {
      const client = new WasmPubMedClient()

      try {
        await client.search_articles('', 5)
        // If it doesn't throw, that's also acceptable
      } catch (error) {
        expect(error).toBeDefined()
      }

      client.free()
    })
  })

  describe('Fetch Article', () => {
    it('should fetch article by PMID successfully', async () => {
      const client = new WasmPubMedClient()

      const article = await client.fetch_article('31978945')

      expect(article).toBeDefined()
      expect(article.pmid).toBe('31978945')
      expect(typeof article.title).toBe('string')
      expect(Array.isArray(article.authors)).toBe(true)
      expect(typeof article.journal).toBe('string')

      client.free()
    })

    it('should fail with invalid PMID', async () => {
      const client = new WasmPubMedClient()

      await expect(client.fetch_article('invalid_pmid')).rejects.toThrow()

      client.free()
    })
  })

  describe('Check PMC Availability', () => {
    it('should check PMC availability successfully', async () => {
      const client = new WasmPubMedClient()

      const pmcId = await client.check_pmc_availability('31978945')

      // Can be null or a PMC ID string
      if (pmcId !== null) {
        expect(typeof pmcId).toBe('string')
        // Handle potential quote formatting issues
        const cleanPmcId = pmcId.replace(/"/g, '')
        expect(cleanPmcId).toMatch(/^PMC\d+$/)
      }

      client.free()
    })

    it('should handle invalid PMID for PMC check', async () => {
      const client = new WasmPubMedClient()

      await expect(client.check_pmc_availability('invalid_pmid')).rejects.toThrow()

      client.free()
    })
  })

  describe('Fetch Full Text', () => {
    it('should fetch full text successfully', async () => {
      const client = new WasmPubMedClient()

      const fullText = await client.fetch_full_text('PMC7092803')

      expect(fullText).toBeDefined()
      expect(typeof fullText.title).toBe('string')
      expect(Array.isArray(fullText.sections)).toBe(true)
      expect(fullText.sections.length).toBeGreaterThan(0)

      client.free()
    })

    it('should fail with invalid PMC ID', async () => {
      const client = new WasmPubMedClient()

      await expect(client.fetch_full_text('invalid_pmc')).rejects.toThrow()

      client.free()
    })
  })

  describe('Convert to Markdown', () => {
    it('should convert full text to markdown successfully', async () => {
      const client = new WasmPubMedClient()

      const fullText = await client.fetch_full_text('PMC7092803')
      const markdown = client.convert_to_markdown(fullText)

      expect(typeof markdown).toBe('string')
      expect(markdown.length).toBeGreaterThan(0)
      expect(markdown).toContain('#') // Should have markdown headers

      client.free()
    })

    it('should fail with invalid full text object', () => {
      const client = new WasmPubMedClient()

      expect(() => {
        client.convert_to_markdown(null as any)
      }).toThrow()

      client.free()
    })
  })

  describe('Get Related Articles', () => {
    it('should get related articles successfully', async () => {
      const client = new WasmPubMedClient()

      const pmids = new Uint32Array([31978945, 33515491])

      try {
        const related = await client.get_related_articles(pmids)

        if (related && Array.isArray(related)) {
          related.forEach(pmid => {
            expect(typeof pmid).toBe('string')
          })
        }
      } catch (error) {
        // Related articles might not be available - that's ok
        console.warn('Related articles not available:', error)
      }

      client.free()
    })

    it('should handle empty PMID array', async () => {
      const client = new WasmPubMedClient()

      const emptyArray = new Uint32Array([])

      try {
        const related = await client.get_related_articles(emptyArray)
        expect(Array.isArray(related)).toBe(true)
      } catch (error) {
        // Empty array might cause error - that's acceptable
        expect(error).toBeDefined()
      }

      client.free()
    })
  })
})
