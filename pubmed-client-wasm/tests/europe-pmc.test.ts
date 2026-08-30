import { describe, expect, it } from 'vitest'
import { WasmPubMedClient } from '../pkg/pubmed_client_wasm.js'

describe('Europe PMC', () => {
  describe('API surface', () => {
    it('should expose the Europe PMC methods', () => {
      const client = new WasmPubMedClient()
      expect(typeof client.europe_pmc_search).toBe('function')
      expect(typeof client.europe_pmc_search_page).toBe('function')
      expect(typeof client.europe_pmc_fetch_full_text).toBe('function')
      expect(typeof client.europe_pmc_fetch_full_text_xml).toBe('function')
      expect(typeof client.europe_pmc_get_references).toBe('function')
      expect(typeof client.europe_pmc_get_citations).toBe('function')
      expect(typeof client.europe_pmc_get_database_links).toBe('function')
      client.free()
    })
  })

  describe('Record addressing', () => {
    // These reject before any request is issued, so they run offline.
    it('should reject an empty id', async () => {
      const client = new WasmPubMedClient()
      await expect(client.europe_pmc_get_references('   ')).rejects.toThrow(/must not be empty/)
      client.free()
    })

    it('should reject a malformed qualified id', async () => {
      const client = new WasmPubMedClient()
      await expect(client.europe_pmc_get_citations('MED/')).rejects.toThrow(/Europe PMC id/)
      client.free()
    })

    it('should reject a non-numeric PMC id', async () => {
      const client = new WasmPubMedClient()
      await expect(client.europe_pmc_fetch_full_text('not-a-pmcid', 'PMC')).rejects.toThrow()
      client.free()
    })

    it('should reject an unknown result type', async () => {
      const client = new WasmPubMedClient()
      await expect(client.europe_pmc_search('cancer', 1, 'verbose')).rejects.toThrow(
        /invalid result_type/
      )
      client.free()
    })
  })

  describe('Search', () => {
    it('should return records across sources', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const results = await client.europe_pmc_search('malaria vaccine', 5)

        expect(Array.isArray(results)).toBe(true)
        expect(results.length).toBeGreaterThan(0)
        const [first] = results
        expect(first.europe_pmc_id).toBe(`${first.source}/${first.id}`)
        // Unmodelled fields cross as a JSON object string.
        expect(() => JSON.parse(first.extra_json)).not.toThrow()
      } finally {
        client.free()
      }
    })

    it('should expose the pagination cursor', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const page = await client.europe_pmc_search_page('malaria vaccine', 'lite', 5)

        expect(Number(page.hit_count)).toBeGreaterThan(0)
        expect(page.results.length).toBeLessThanOrEqual(5)
        expect(page.next_cursor_mark).toBeTruthy()
      } finally {
        client.free()
      }
    })
  })

  describe('Records', () => {
    it('should fetch parsed full text for a PMC record', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const article = await client.europe_pmc_fetch_full_text('PMC3258128')
        expect(article.pmcid).toBe('PMC3258128')
      } finally {
        client.free()
      }
    })

    it('should fetch raw JATS XML', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const xml = await client.europe_pmc_fetch_full_text_xml('PMC3258128')
        expect(xml).toContain('<article')
      } finally {
        client.free()
      }
    })

    it('should list cited works', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const references = await client.europe_pmc_get_references('PMC3258128')
        expect(references.length).toBeGreaterThan(0)
      } finally {
        client.free()
      }
    })

    it('should list citing articles', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const citations = await client.europe_pmc_get_citations('PMC3258128')
        expect(citations.length).toBeGreaterThan(0)
      } finally {
        client.free()
      }
    })

    it('should list external database cross-references', async () => {
      const client = WasmPubMedClient.new_for_testing()
      try {
        const links = await client.europe_pmc_get_database_links('PMC3258128')
        // A record may legitimately have none; assert on shape, not presence.
        for (const link of links) {
          expect(Array.isArray(link.info)).toBe(true)
        }
      } finally {
        client.free()
      }
    })
  })
})
