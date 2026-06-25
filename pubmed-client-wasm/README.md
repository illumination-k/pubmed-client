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

- Check PMC availability and Open Access (OA) subset status
- Retrieve structured full-text content
- Convert articles to Markdown format (with configurable options)

### Query Builder

- Fluent `WasmSearchQuery` builder with field filters (title, author, MeSH, journal, ORCID, …)
- Date-range, article-type, language, and study-population filters
- Boolean composition: AND / OR / NOT / exclude / grouping

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

##### fetch_pmc_as_markdown(pmcid, options?)

Fetch a PMC article and convert it to Markdown in a single call.

```javascript
const markdown = await client.fetch_pmc_as_markdown("PMC7239045", {
    include_metadata: true,
    include_toc: true,
    use_yaml_frontmatter: true,
    include_orcid_links: false,
    include_figure_captions: true,
});
console.log(markdown);
```

**Parameters:**

- `pmcid` (string): PMC ID
- `options` (object, optional): Markdown conversion options. All fields are
  optional booleans; unset fields fall back to converter defaults:
  `include_metadata`, `include_toc`, `use_yaml_frontmatter`,
  `include_orcid_links`, `include_figure_captions`.

**Returns:** Promise<string>

##### is_oa_subset(pmcid)

Check whether a PMC article is in the Open Access (OA) subset (i.e. has
programmatic full-text access).

```javascript
const info = await client.is_oa_subset("PMC7239045");
if (info.is_oa_subset) {
    console.log(`License: ${info.license}`);
    console.log(`Download: ${info.download_link}`);
} else {
    console.log(`Not in OA subset: ${info.error_code}`);
}
```

**Parameters:**

- `pmcid` (string): PMC ID

**Returns:** Promise<OaSubsetInfo>

##### get_related_articles(pmids)

Find articles related to given PMIDs.

```javascript
const related = await client.get_related_articles([31978945, 33515491]);
console.log(`Found ${related.related_pmids.length} related articles`);
```

**Parameters:**

- `pmids` (number[]): Array of PubMed IDs

**Returns:** Promise<RelatedArticles>

### WasmSearchQuery

A fluent builder for constructing complex PubMed queries with field filters,
date ranges, article types, and boolean composition. Build a query string with
`build()`, or run it directly with `search_and_fetch(client, limit)`.

```javascript
const { WasmSearchQuery, WasmPubMedClient } = require('pubmed-client-wasm');

const client = new WasmPubMedClient();

const query = new WasmSearchQuery()
    .title_abstract("CRISPR")
    .mesh_term("Neoplasms")
    .author("Zhang Y")
    .article_types_str(["Review", "Meta-Analysis"])
    .published_between(2019, 2023)
    .humans_only();

console.log(query.build());
// (CRISPR[tiab]) AND ... AND (2019:2023[pdat]) ...

const articles = await query.search_and_fetch(client, 20);
```

#### Field filters

- `query(text)`, `terms(string[])`, `custom_filter(text)`
- `title(text)`, `abstract_contains(text)`, `title_abstract(text)`, `has_abstract()`
- `author(name)`, `first_author(name)`, `last_author(name)`, `affiliation(institution)`, `orcid(id)`
- `journal(name)`, `journal_abbreviation(abbrev)`, `grant_number(gr)`, `isbn(isbn)`, `issn(issn)`
- `mesh_term(term)`, `mesh_terms(string[])`, `mesh_major_topic(term)`, `mesh_subheading(sh)`

#### Filters & study population

- `free_full_text_only()`, `full_text_only()`, `pmc_only()`
- `humans_only()`, `animal_studies_only()`, `age_group(group)`, `organism_mesh(organism)`
- `article_type_str(type)`, `article_types_str(string[])`, `language_str(lang)`, `sort_str(order)`

#### Dates (years must be 1800–3000)

- `published_after(year)`, `published_before(year)`, `published_in_year(year)`
- `published_between(startYear, endYear?)`, `date_range(startYear, endYear?)`

#### Boolean composition

These return a **new** query and do not consume their operands:

- `and(other)`, `or(other)`, `negate()`, `exclude(other)`, `group()`

```javascript
const a = new WasmSearchQuery().query("diabetes");
const b = new WasmSearchQuery().query("hypertension");
a.or(b).build(); // "(diabetes) OR (hypertension)"
```

#### Misc

- `limit(n)` / `get_limit()`, `build()`, `search_and_fetch(client, limit)`

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

### OaSubsetInfo

```typescript
interface OaSubsetInfo {
    pmcid: string;
    is_oa_subset: boolean;
    citation?: string;
    license?: string;
    retracted: boolean;
    download_link?: string;
    download_format?: string;
    updated?: string;
    error_code?: string;
    error_message?: string;
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

## Publishing to npm

This package can be published to npm registry. Follow these steps:

### Prerequisites

1. Create an npm account at [npmjs.com](https://www.npmjs.com)
2. Install npm CLI and login: `npm login`
3. Ensure you have permission to publish the package name

### Publishing Steps

#### Method 1: Using wasm-pack (Recommended)

```bash
# Build and publish in one command
wasm-pack publish --access public
```

#### Method 2: Manual Publishing

```bash
# 1. Build the WASM package
pnpm run build

# 2. Navigate to pkg directory and publish
cd pkg
npm publish --access public
```

### Pre-publish Checklist

Before publishing, ensure:

- [ ] Version number is updated in both `package.json` and `Cargo.toml`
- [ ] All tests pass: `pnpm run test`
- [ ] WASM builds successfully: `pnpm run build`
- [ ] Documentation is up to date
- [ ] `.npmignore` excludes unnecessary files

### Version Management

```bash
# Update version in both files
# package.json
npm version patch|minor|major

# Cargo.toml (update manually or use cargo-edit)
cargo install cargo-edit
cargo set-version --bump patch|minor|major
```

### Package Verification

After publishing, verify the package:

```bash
# Install from npm
npm install pubmed-client-wasm

# Test basic functionality
node -e "const client = require('pubmed-client-wasm'); console.log('✅ Package installed successfully');"
```

## License

MIT OR Apache-2.0

## Contributing

See the main repository for contribution guidelines and development setup.
