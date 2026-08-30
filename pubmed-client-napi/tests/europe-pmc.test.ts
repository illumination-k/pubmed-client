import { describe, expect, it } from 'vitest'
import { PubMedClient } from '../index.js'
import { isNetworkError } from './setup'

describe('Europe PMC', () => {
  describe('API surface', () => {
    it('should expose the Europe PMC methods', () => {
      const client = new PubMedClient()
      expect(typeof client.europePmcSearch).toBe('function')
      expect(typeof client.europePmcSearchPage).toBe('function')
      expect(typeof client.europePmcFetchFullText).toBe('function')
      expect(typeof client.europePmcFetchFullTextXml).toBe('function')
      expect(typeof client.europePmcGetReferences).toBe('function')
      expect(typeof client.europePmcGetCitations).toBe('function')
      expect(typeof client.europePmcGetDatabaseLinks).toBe('function')
      expect(typeof client.europePmcDownloadSupplementaryFiles).toBe('function')
    })
  })

  describe('Record addressing', () => {
    // These reject before any request is issued, so they run offline.
    it('should reject an empty id', async () => {
      const client = new PubMedClient()
      await expect(client.europePmcGetReferences('   ')).rejects.toThrow(/must not be empty/)
    })

    it('should reject a malformed qualified id', async () => {
      const client = new PubMedClient()
      await expect(client.europePmcGetCitations('MED/')).rejects.toThrow(/invalid Europe PMC id/)
    })

    it('should reject a non-numeric PMC id', async () => {
      const client = new PubMedClient()
      await expect(client.europePmcFetchFullText('not-a-pmcid', 'PMC')).rejects.toThrow(
        /invalid PMC id/
      )
    })

    it('should reject an unknown result type', async () => {
      const client = new PubMedClient()
      await expect(client.europePmcSearch('cancer', 1, 'verbose')).rejects.toThrow(
        /invalid resultType/
      )
    })
  })

  describe('Search', () => {
    it('should return records across sources', async () => {
      const client = new PubMedClient()
      try {
        const results = await client.europePmcSearch('malaria vaccine', 5)

        expect(results.length).toBeGreaterThan(0)
        expect(results.length).toBeLessThanOrEqual(5)
        const [first] = results
        expect(first.id.length).toBeGreaterThan(0)
        expect(first.source.length).toBeGreaterThan(0)
        expect(first.europePmcId).toBe(`${first.source}/${first.id}`)
        // Unmodelled fields cross as a JSON object string.
        expect(() => JSON.parse(first.extraJson)).not.toThrow()
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })

    it('should expose the pagination cursor', async () => {
      const client = new PubMedClient()
      try {
        const page = await client.europePmcSearchPage('malaria vaccine', 'lite', 5)

        expect(page.hitCount).toBeGreaterThan(0)
        expect(page.results.length).toBeLessThanOrEqual(5)
        expect(page.nextCursorMark).toBeTruthy()
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })
  })

  describe('Records', () => {
    it('should fetch parsed full text for a PMC record', async () => {
      const client = new PubMedClient()
      try {
        const article = await client.europePmcFetchFullText('PMC3258128')

        expect(article.pmcid).toBe('PMC3258128')
        expect(article.title).toBeTruthy()
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })

    it('should fetch raw JATS XML', async () => {
      const client = new PubMedClient()
      try {
        const xml = await client.europePmcFetchFullTextXml('PMC3258128')
        expect(xml).toContain('<article')
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })

    it('should list cited works', async () => {
      const client = new PubMedClient()
      try {
        const references = await client.europePmcGetReferences('PMC3258128')
        expect(references.length).toBeGreaterThan(0)
        expect(references.some(reference => reference.title)).toBe(true)
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })

    it('should list citing articles', async () => {
      const client = new PubMedClient()
      try {
        const citations = await client.europePmcGetCitations('PMC3258128')
        expect(citations.length).toBeGreaterThan(0)
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })

    it('should list external database cross-references', async () => {
      const client = new PubMedClient()
      try {
        const links = await client.europePmcGetDatabaseLinks('PMC3258128')
        // A record may legitimately have none; assert on shape, not presence.
        for (const link of links) {
          expect(link.dbName).toBeTruthy()
          expect(Array.isArray(link.info)).toBe(true)
        }
      } catch (error) {
        if (isNetworkError(error)) return
        throw error
      }
    })
  })
})
