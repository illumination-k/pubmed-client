import { describe, expect, it } from 'vitest'
import { ArticleIterator, PubMedClient } from '../index.js'
import { searchAll } from '../streaming.js'
import { isNetworkError, TEST_CONFIG } from './setup'

describe('Streaming Search', () => {
  describe('ArticleIterator', () => {
    it('should create iterator with searchWithHistory', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory(TEST_CONFIG.TEST_QUERY, 5)

        expect(iterator).toBeDefined()
        expect(typeof iterator.next).toBe('function')
        expect(typeof iterator.totalCount).toBe('number')
        expect(typeof iterator.progress).toBe('number')
        expect(typeof iterator.isDone).toBe('boolean')
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should return total count', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory('cancer', 5)

        expect(iterator.totalCount).toBeGreaterThan(0)
        expect(iterator.progress).toBe(0)
        expect(iterator.isDone).toBe(false)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should iterate through articles with next()', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory(TEST_CONFIG.TEST_QUERY, 5)
        const articles = []

        let article = await iterator.next()
        while (article !== null && articles.length < 10) {
          articles.push(article)
          article = await iterator.next()
        }

        expect(articles.length).toBeGreaterThan(0)
        expect(articles.length).toBeLessThanOrEqual(10)

        for (const a of articles) {
          expect(a.pmid).toBeDefined()
          expect(typeof a.pmid).toBe('string')
          expect(a.title).toBeDefined()
        }
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should track progress correctly', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory('diabetes', 5)
        const initialProgress = iterator.progress

        const article = await iterator.next()
        if (article !== null) {
          expect(iterator.progress).toBeGreaterThan(initialProgress)
        }
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should handle empty results', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory('xyznonexistentquery12345', 5)

        // Empty results should have totalCount 0 or isDone true
        if (iterator.totalCount === 0) {
          expect(iterator.isDone).toBe(true)
        }

        const article = await iterator.next()
        expect(article).toBeNull()
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })
  })

  describe('AsyncIterator Support', () => {
    it('should support for await...of syntax', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory(TEST_CONFIG.TEST_QUERY, 5)
        const articles = []

        for await (const article of iterator) {
          articles.push(article)
          if (articles.length >= 5) break
        }

        expect(articles.length).toBeGreaterThan(0)
        expect(articles.length).toBeLessThanOrEqual(5)

        for (const a of articles) {
          expect(a.pmid).toBeDefined()
          expect(a.title).toBeDefined()
        }
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should support early break in for await...of', async () => {
      const client = new PubMedClient()

      try {
        const iterator = await client.searchWithHistory('cancer', 10)
        let count = 0

        for await (const article of iterator) {
          count++
          expect(article).toBeDefined()
          if (count >= 3) break
        }

        expect(count).toBe(3)
        // After breaking, isDone may or may not be true depending on state
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })
  })

  describe('searchAll helper function', () => {
    it('should stream articles with searchAll', async () => {
      const client = new PubMedClient()

      try {
        const articles = []

        for await (const article of searchAll(client, TEST_CONFIG.TEST_QUERY, {
          batchSize: 5,
        })) {
          articles.push(article)
          if (articles.length >= 5) break
        }

        expect(articles.length).toBeGreaterThan(0)
        expect(articles.length).toBeLessThanOrEqual(5)

        for (const a of articles) {
          expect(a.pmid).toBeDefined()
          expect(a.title).toBeDefined()
        }
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })

    it('should use default batchSize', async () => {
      const client = new PubMedClient()

      try {
        const articles = []

        for await (const article of searchAll(client, 'genomics')) {
          articles.push(article)
          if (articles.length >= 3) break
        }

        expect(articles.length).toBe(3)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })
  })

  describe('Batch Fetching', () => {
    it('should fetch multiple batches', async () => {
      const client = new PubMedClient()

      try {
        // Use a small batch size to force multiple fetches
        const iterator = await client.searchWithHistory('cancer treatment', 3)

        if (iterator.totalCount < 6) {
          console.warn('Not enough results to test multiple batches')
          return
        }

        const articles = []
        for await (const article of iterator) {
          articles.push(article)
          if (articles.length >= 6) break
        }

        // Should have fetched at least 2 batches worth
        expect(articles.length).toBe(6)

        // All PMIDs should be unique
        const pmids = articles.map((a) => a.pmid)
        const uniquePmids = new Set(pmids)
        expect(uniquePmids.size).toBe(pmids.length)
      } catch (error) {
        if (isNetworkError(error)) {
          console.warn('Skipping test due to API/network issue:', error)
          return
        }
        throw error
      }
    })
  })
})
