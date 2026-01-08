// Type definitions for streaming support
import { Article, ArticleIterator, PubMedClient, SearchQuery } from './index';

export interface SearchAllOptions {
  /** Number of articles to fetch per batch (default: 100) */
  batchSize?: number;
}

/**
 * Async generator for streaming all search results
 *
 * @param client - The PubMed client instance
 * @param query - Search query string
 * @param options - Options for the search
 * @yields Each article from the search results
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
export function searchAll(
  client: PubMedClient,
  query: string,
  options?: SearchAllOptions
): AsyncGenerator<Article, void, unknown>;

// Augment ArticleIterator to include AsyncIterator protocol
declare module './index' {
  interface ArticleIterator {
    [Symbol.asyncIterator](): AsyncIterator<Article>;
  }
}

// Re-export everything from the main module
export { Article, ArticleIterator, PubMedClient, SearchQuery } from './index';
