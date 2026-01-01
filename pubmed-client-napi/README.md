# PubMed Client - Node.js Native Bindings

[![npm version](https://badge.fury.io/js/pubmed-client.svg)](https://www.npmjs.com/package/pubmed-client)
[![Node.js](https://img.shields.io/badge/node-%3E%3D16-blue.svg)](https://nodejs.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Native Node.js bindings for the PubMed and PMC (PubMed Central) API client, built with [napi-rs](https://napi.rs/).

## Overview

This package provides high-performance native Node.js bindings to the Rust-based PubMed client library. Unlike WASM bindings, native bindings offer better performance and seamless integration with the Node.js ecosystem.

## Features

- **PubMed Search**: Search and retrieve article metadata from PubMed
- **PMC Full-Text**: Access full-text articles from PubMed Central
- **Markdown Conversion**: Convert PMC articles to well-formatted Markdown
- **SearchQuery Builder**: Build complex queries programmatically with filters
- **High Performance**: Native Rust bindings via napi-rs
- **TypeScript Support**: Full type definitions included
- **Cross-Platform**: Pre-built binaries for Windows, macOS, and Linux (x64/ARM64)

## Installation

```bash
npm install pubmed-client
# or
pnpm add pubmed-client
# or
yarn add pubmed-client
```

## Quick Start

### Basic Search

```typescript
import { PubMedClient } from 'pubmed-client';

const client = new PubMedClient();

// Search for articles
const articles = await client.search('COVID-19 vaccine', 10);

for (const article of articles) {
  console.log(`[${article.pmid}] ${article.title}`);
  console.log(`  Journal: ${article.journal}`);
  console.log(`  Authors: ${article.authors.map(a => a.fullName).join(', ')}`);
}
```

### With Configuration

```typescript
import { PubMedClient, type Config } from 'pubmed-client';

const config: Config = {
  apiKey: process.env.NCBI_API_KEY,     // Optional: increases rate limit to 10 req/s
  email: 'your-email@example.com',       // Recommended by NCBI
  tool: 'my-app',                        // Your application name
  timeoutSeconds: 30,                    // Request timeout
};

const client = PubMedClient.withConfig(config);
```

### Fetch Single Article

```typescript
const article = await client.fetchArticle('31978945');

console.log(`Title: ${article.title}`);
console.log(`Abstract: ${article.abstractText}`);
console.log(`DOI: ${article.doi}`);
console.log(`Keywords: ${article.keywords.join(', ')}`);
```

### Fetch PMC Full-Text

```typescript
// Check if PMC full-text is available
const pmcId = await client.checkPmcAvailability('31978945');

if (pmcId) {
  // Fetch full-text article
  const fullText = await client.fetchPmcArticle(pmcId);

  console.log(`Title: ${fullText.title}`);
  console.log(`Sections: ${fullText.sections.length}`);
  console.log(`References: ${fullText.references.length}`);

  // Access sections
  for (const section of fullText.sections) {
    console.log(`Section: ${section.title}`);
    console.log(`Content: ${section.content.substring(0, 200)}...`);
  }
}
```

### Convert to Markdown

```typescript
import { type MarkdownOptions } from 'pubmed-client';

const options: MarkdownOptions = {
  includeMetadata: true,
  includeToc: true,
  useYamlFrontmatter: true,
  includeOrcidLinks: true,
  includeFigureCaptions: true,
};

const markdown = await client.fetchPmcAsMarkdown('PMC7906746', options);
console.log(markdown);
```

### Using SearchQuery Builder

The `SearchQuery` builder provides a fluent API for constructing complex PubMed queries:

```typescript
import { PubMedClient, SearchQuery } from 'pubmed-client';

const client = new PubMedClient();

// Build a complex query
const query = new SearchQuery()
  .query('cancer')
  .publishedBetween(2020, 2024)
  .articleType('Clinical Trial')
  .freeFullTextOnly()
  .setLimit(50);

// Execute the query
const articles = await client.executeQuery(query);
```

#### Date Filtering

```typescript
// Published in a specific year
const query = new SearchQuery()
  .query('diabetes')
  .publishedInYear(2024);

// Published between years
const query2 = new SearchQuery()
  .query('treatment')
  .publishedBetween(2020, 2024);

// Published after a year
const query3 = new SearchQuery()
  .query('CRISPR')
  .publishedAfter(2020);
```

#### Article Type Filtering

```typescript
// Supported types: "Clinical Trial", "Review", "Systematic Review",
// "Meta-Analysis", "Case Reports", "RCT", "Observational Study"

const query = new SearchQuery()
  .query('cancer')
  .articleTypes(['RCT', 'Meta-Analysis']);
```

#### MeSH Terms and Advanced Filters

```typescript
const query = new SearchQuery()
  .meshTerm('Neoplasms')
  .meshMajorTopic('Diabetes Mellitus, Type 2')
  .meshSubheading('drug therapy')
  .author('Smith J')
  .firstAuthor('Williams K')
  .affiliation('Harvard Medical School')
  .journal('Nature')
  .language('English')
  .humanStudiesOnly()
  .hasAbstract();
```

#### Boolean Logic

```typescript
// AND combination
const q1 = new SearchQuery().query('covid-19');
const q2 = new SearchQuery().query('vaccine');
const combined = q1.and(q2);
combined.build(); // "(covid-19) AND (vaccine)"

// OR combination
const either = q1.or(q2);
either.build(); // "(covid-19) OR (vaccine)"

// Exclusion
const base = new SearchQuery().query('cancer treatment');
const exclude = new SearchQuery().query('animal studies');
const filtered = base.exclude(exclude);
filtered.build(); // "(cancer treatment) NOT (animal studies)"
```

## API Reference

### PubMedClient

| Method                                | Description                               |
| ------------------------------------- | ----------------------------------------- |
| `new PubMedClient()`                  | Create client with default configuration  |
| `PubMedClient.withConfig(config)`     | Create client with custom configuration   |
| `search(query, limit?)`               | Search PubMed and return articles         |
| `fetchArticle(pmid)`                  | Fetch single article by PMID              |
| `fetchPmcArticle(pmcid)`              | Fetch full-text article from PMC          |
| `fetchPmcAsMarkdown(pmcid, options?)` | Fetch PMC article as Markdown             |
| `checkPmcAvailability(pmid)`          | Check if PMC full-text is available       |
| `executeQuery(searchQuery)`           | Execute a SearchQuery and return articles |

### SearchQuery Builder

| Method                          | Description                       |
| ------------------------------- | --------------------------------- |
| `query(term)`                   | Add search term                   |
| `terms(terms[])`                | Add multiple search terms         |
| `setLimit(n)`                   | Set maximum results               |
| `build()`                       | Build final query string          |
| `publishedInYear(year)`         | Filter by publication year        |
| `publishedBetween(start, end?)` | Filter by date range              |
| `publishedAfter(year)`          | Filter to articles after year     |
| `publishedBefore(year)`         | Filter to articles before year    |
| `articleType(type)`             | Filter by article type            |
| `articleTypes(types[])`         | Filter by multiple article types  |
| `language(lang)`                | Filter by language                |
| `freeFullTextOnly()`            | Filter to open access articles    |
| `fullTextOnly()`                | Filter to articles with full text |
| `pmcOnly()`                     | Filter to PMC articles            |
| `hasAbstract()`                 | Filter to articles with abstracts |
| `titleContains(text)`           | Search in titles                  |
| `abstractContains(text)`        | Search in abstracts               |
| `titleOrAbstract(text)`         | Search in title or abstract       |
| `journal(name)`                 | Filter by journal                 |
| `meshTerm(term)`                | Filter by MeSH term               |
| `meshMajorTopic(term)`          | Filter by MeSH major topic        |
| `meshTerms(terms[])`            | Filter by multiple MeSH terms     |
| `author(name)`                  | Filter by author                  |
| `firstAuthor(name)`             | Filter by first author            |
| `lastAuthor(name)`              | Filter by last author             |
| `affiliation(institution)`      | Filter by affiliation             |
| `orcid(id)`                     | Filter by ORCID                   |
| `humanStudiesOnly()`            | Filter to human studies           |
| `animalStudiesOnly()`           | Filter to animal studies          |
| `and(other)`                    | Combine with AND logic            |
| `or(other)`                     | Combine with OR logic             |
| `exclude(other)`                | Exclude matching articles         |
| `negate()`                      | Negate the query                  |
| `group()`                       | Add parentheses for grouping      |

## Data Types

### Article

```typescript
interface Article {
  pmid: string;
  title: string;
  authors: Author[];
  journal: string;
  pubDate: string;
  doi?: string;
  pmcId?: string;
  abstractText?: string;
  articleTypes: string[];
  keywords: string[];
}
```

### FullTextArticle

```typescript
interface FullTextArticle {
  pmcid: string;
  pmid?: string;
  title: string;
  authors: Author[];
  journal: string;
  pubDate: string;
  doi?: string;
  sections: Section[];
  references: Reference[];
  keywords: string[];
}
```

### Author

```typescript
interface Author {
  fullName: string;
  orcid?: string;
  affiliation?: string;
}
```

## Platform Support

Pre-built binaries are available for:

| Platform      | Architecture               |
| ------------- | -------------------------- |
| Windows       | x64                        |
| macOS         | x64, ARM64 (Apple Silicon) |
| Linux (glibc) | x64, ARM64                 |
| Linux (musl)  | x64, ARM64                 |

## Development

### Prerequisites

- Node.js >= 16
- Rust toolchain
- pnpm

### Setup

```bash
cd pubmed-client-napi
pnpm install
```

### Build

```bash
# Development build
pnpm run build:debug

# Release build
pnpm run build
```

### Test

```bash
pnpm test
```

### Code Quality

```bash
pnpm run lint       # Run linter
pnpm run format     # Format code
pnpm run typecheck  # TypeScript check
```

## License

MIT

## Links

- [Repository](https://github.com/illumination-k/pubmed-client)
- [npm Package](https://www.npmjs.com/package/pubmed-client)
- [Core Rust Library](../pubmed-client)
- [WebAssembly Bindings](../pubmed-client-wasm)
- [Python Bindings](../pubmed-client-py)
- [NCBI E-utilities Documentation](https://www.ncbi.nlm.nih.gov/books/NBK25501/)
