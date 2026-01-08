// Streaming support for PubMed client
// This file adds AsyncIterator support for ArticleIterator

import { PubMedClient, SearchQuery, ArticleIterator } from './index.js';

// Add Symbol.asyncIterator support to ArticleIterator
if (typeof ArticleIterator !== 'undefined') {
  ArticleIterator.prototype[Symbol.asyncIterator] = function () {
    const iterator = this;
    return {
      async next() {
        const article = await iterator.next();
        if (article === null) {
          return { done: true, value: undefined };
        }
        return { done: false, value: article };
      },
    };
  };
}

/**
 * Async generator for streaming all search results
 *
 * @param {PubMedClient} client - The PubMed client instance
 * @param {string} query - Search query string
 * @param {Object} [options] - Options for the search
 * @param {number} [options.batchSize=100] - Number of articles per batch
 * @yields {Article} Each article from the search results
 *
 * @example
 * ```typescript
 * import { PubMedClient, searchAll } from 'pubmed-client/streaming';
 *
 * const client = new PubMedClient();
 * for await (const article of searchAll(client, 'COVID-19 vaccine')) {
 *   console.log(article.title);
 *   if (someCondition) break;
 * }
 * ```
 */
export async function* searchAll(client, query, options = {}) {
  const batchSize = options.batchSize || 100;
  const iterator = await client.searchWithHistory(query, batchSize);

  for await (const article of iterator) {
    yield article;
  }
}

// Re-export everything from the main module
export { PubMedClient, SearchQuery, ArticleIterator };
