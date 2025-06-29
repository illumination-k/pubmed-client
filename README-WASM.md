# PubMed Client WebAssembly

WebAssembly bindings for the PubMed client library, enabling biomedical research data access in Node.js and browsers.

## Installation

```bash
npm install pubmed-client-wasm
```

## Quick Start

```javascript
const { WasmPubMedClient, WasmClientConfig } = require('pubmed-client-wasm');

async function searchArticles() {
    // Create configuration
    const config = new WasmClientConfig();
    config.set_email("your.email@institution.edu");
    config.set_api_key("your_ncbi_api_key"); // Optional but recommended

    // Create client
    const client = WasmPubMedClient.with_config(config);

    // Search for articles
    const articles = await client.search_articles("covid-19 treatment", 10);

    console.log(`Found ${articles.length} articles:`);
    articles.forEach(article => {
        console.log(`- ${article.title}`);
        console.log(`  Authors: ${article.authors.join(', ')}`);
        console.log(`  Journal: ${article.journal} (${article.pub_date})`);
    });
}

searchArticles().catch(console.error);
```

## Features

### Article Search and Retrieval

- Search PubMed database with queries
- Fetch detailed article metadata
- Access author information, abstracts, and bibliographic data

### PMC Full-Text Access

- Check PMC availability for articles
- Retrieve structured full-text content
- Convert articles to Markdown format

### Advanced Features

- Rate limiting compliance with NCBI guidelines
- Configurable API keys and request settings
- TypeScript definitions included

## API Reference

### WasmClientConfig

Configuration object for the PubMed client.

```javascript
const config = new WasmClientConfig();
config.set_api_key("your_api_key");        // NCBI API key (recommended)
config.set_email("you@example.com");       // Contact email (required)
config.set_tool("Your App Name");          // Application name
config.set_rate_limit(10.0);              // Requests per second
config.set_timeout_seconds(30);           // Request timeout
```

### WasmPubMedClient

Main client for interacting with PubMed and PMC APIs.

#### Constructor

```javascript
// Default configuration
const client = new WasmPubMedClient();

// With custom configuration
const client = WasmPubMedClient.with_config(config);
```

#### Methods

##### search_articles(query, limit)

Search for articles and return metadata.

```javascript
const articles = await client.search_articles("machine learning", 20);
```

**Parameters:**

- `query` (string): Search query
- `limit` (number): Maximum number of results

**Returns:** Promise<Article[]>

##### fetch_article(pmid)

Fetch detailed information for a specific article.

```javascript
const article = await client.fetch_article("31978945");
```

**Parameters:**

- `pmid` (string): PubMed ID

**Returns:** Promise<Article>

##### check_pmc_availability(pmid)

Check if PMC full-text is available for an article.

```javascript
const pmcid = await client.check_pmc_availability("31978945");
if (pmcid) {
    console.log(`PMC ID: ${pmcid}`);
}
```

**Parameters:**

- `pmid` (string): PubMed ID

**Returns:** Promise<string | null>

##### fetch_full_text(pmcid)

Retrieve structured full-text from PMC.

```javascript
const fullText = await client.fetch_full_text("PMC7239045");
console.log(`Title: ${fullText.title}`);
console.log(`Sections: ${fullText.sections.length}`);
```

**Parameters:**

- `pmcid` (string): PMC ID

**Returns:** Promise<FullText>

##### convert_to_markdown(fullText)

Convert PMC full-text to Markdown format.

```javascript
const fullText = await client.fetch_full_text("PMC7239045");
const markdown = client.convert_to_markdown(fullText);
console.log(markdown);
```

**Parameters:**

- `fullText` (FullText): Full-text object from fetch_full_text

**Returns:** string

##### get_related_articles(pmids)

Find articles related to given PMIDs.

```javascript
const related = await client.get_related_articles([31978945, 33515491]);
console.log(`Found ${related.related_pmids.length} related articles`);
```

**Parameters:**

- `pmids` (number[]): Array of PubMed IDs

**Returns:** Promise<RelatedArticles>

## Data Types

### Article

```typescript
interface Article {
    pmid: string;
    title: string;
    authors: string[];
    journal: string;
    pub_date: string;
    abstract_text?: string;
    doi?: string;
    article_types: string[];
}
```

### FullText

```typescript
interface FullText {
    pmcid: string;
    pmid?: string;
    title: string;
    authors: Author[];
    journal: Journal;
    pub_date: string;
    doi?: string;
    sections: Section[];
    references: Reference[];
    article_type?: string;
    keywords: string[];
}
```

### Author

```typescript
interface Author {
    given_names?: string;
    surname?: string;
    full_name: string;
    email?: string;
    affiliations: string[];
    is_corresponding: boolean;
}
```

### Section

```typescript
interface Section {
    section_type: string;
    title?: string;
    content: string;
}
```

## Error Handling

All async methods return Promises that may reject with error messages:

```javascript
try {
    const articles = await client.search_articles("invalid query", 10);
} catch (error) {
    console.error(`Search failed: ${error.message}`);
}
```

## Rate Limiting

The client automatically handles rate limiting according to NCBI guidelines:

- 3 requests/second without API key
- 10 requests/second with API key

## Requirements

- Node.js 14.0.0 or higher
- WebAssembly support (available in all modern Node.js versions)

## Browser Support

The library can also be used in browsers with the web target:

```javascript
import init, { WasmPubMedClient } from 'pubmed-client-wasm/web';

async function initClient() {
    await init(); // Initialize WASM module
    const client = new WasmPubMedClient();
    // Use client...
}
```

## License

MIT OR Apache-2.0

## Contributing

See the main repository for contribution guidelines and development setup.
