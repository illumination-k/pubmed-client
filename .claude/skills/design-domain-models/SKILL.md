---
name: design-domain-models
description: Design and evolve domain models for the pubmed-client-rs workspace. Use when adding new types, modifying existing models, or planning domain model migrations. Ensures DTD faithfulness, domain purity, backward compatibility, and DDD principles.
---

# Domain Model Design

## Overview

Guide the design of domain models for `pubmed-parser` that are DTD-faithful, pure, and safe to introduce alongside existing applications. This project is a text mining toolkit — domain models must faithfully represent the structure of biomedical literature as defined by JATS/NLM DTDs, independent of persistence, infrastructure, or application concerns.

## When to Use This Skill

Use this skill when:

- Adding new types or fields to domain models (`domain.rs`)
- Designing a new domain type for a concept not yet modeled
- Deciding where a new concept belongs (common, pubmed, pmc)
- Reviewing whether a proposed model change breaks existing consumers
- Planning how to expose new domain types through the crate layers

## Core Principles

### 1. DTD Faithfulness

Domain models MUST reflect the source DTD structure. For PMC, this is **JATS Archiving and Interchange 1.4**.

**Rules**:

- Every domain struct field must map to a DTD element or attribute
- Field names should mirror DTD element names (Rust snake_case of the XML element)
- Document the DTD source in field comments: `/// From '<article-title>'`
- Do NOT add fields that are inferred, computed, or derived from the XML (e.g., `file_type` inferred from URL extension)
- Do NOT add fields for extraction/runtime artifacts (e.g., `file_path`, `extracted_file_path`)
- Optional DTD elements → `Option<T>`, repeatable elements → `Vec<T>`

**Validation Workflow**:

1. Check the JATS DTD for the element: https://jats.nlm.nih.gov/archiving/tag-library/1.4/
2. Identify parent element, cardinality (0..1, 0..n, 1, 1..n), and attributes
3. Map to Rust types following existing conventions
4. Document the mapping in code comments

**Example**:

```rust
/// PMC article structured from JATS `<article>` element.
///
/// DTD: https://jats.nlm.nih.gov/archiving/tag-library/1.4/element/article.html
pub struct PmcArticle {
    /// From `<article-id pub-id-type="pmc">`
    pub pmcid: PmcId,
    /// From `<article-id pub-id-type="pmid">`
    pub pmid: Option<PubMedId>,
    /// From `<article-title>`
    pub title: String,
    // ...
}
```

### 2. Domain Model Purity

Domain models MUST be pure — they represent **what the data is**, not how it's stored, transported, or used.

**DO**:

- Use semantic Rust types (`PmcId`, `PubMedId`, enums with variants)
- Model the domain as it exists in the DTD/specification
- Keep models in `pubmed-parser` (the pure parsing crate, no I/O)
- Derive only: `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`
- Add domain-level methods (queries, transformations on the data itself)

**DO NOT**:

- Add database columns, primary keys, or ORM annotations
- Add `created_at`, `updated_at`, or persistence timestamps
- Add serialization format hints (`#[serde(rename = "...")]` for non-XML purposes)
- Add HTTP/API response wrappers or status codes
- Add caching keys or TTL fields
- Reference external services, file system paths, or network URLs (except those in the DTD itself like `xlink:href`)
- Add feature flags or configuration to domain types

### 3. Backward Compatibility — Do Not Break Existing Applications

Domain models are a new layer that will eventually replace parser models. During this transition, they MUST NOT break existing consumers.

**Rules**:

- Adding new domain types is always safe (additive change)
- Adding new `parse_*_domain()` functions alongside existing ones is safe
- NEVER remove or rename existing public parser functions (`parse_pmc_xml`, `parse_article_from_xml`)
- NEVER change the return type of existing public functions
- NEVER remove existing re-exports from `pubmed-client/src/lib.rs`
- New domain types can be re-exported alongside existing types — use distinct names to avoid conflicts
- `TryFrom<ParserModel> for DomainModel` bridges both worlds without modifying the parser model

**Safe introduction pattern**:

```rust
// Existing function — untouched
pub fn parse_pmc_xml(xml: &str, pmcid: &str) -> Result<PmcFullText> { ... }

// New function — added alongside
pub fn parse_pmc_xml_domain(xml: &str, pmcid: &str) -> Result<PmcArticle> {
    let parsed = parse_pmc_xml(xml, pmcid)?;
    PmcArticle::try_from(parsed).map_err(Into::into)
}
```

### 4. DDD: Aggregates, Entities, and Value Objects

**Aggregates** — Each top-level document is an aggregate root. Access internal objects through the aggregate root:

| Aggregate Root  | Bounded Context | Identity   |
| --------------- | --------------- | ---------- |
| `PmcArticle`    | PMC full-text   | `PmcId`    |
| `PubMedArticle` | PubMed metadata | `PubMedId` |

**Value Objects** — Types without independent identity, defined by their content:

- `Author`, `Affiliation` — same name + affiliation = same author
- `PublicationDate`, `Section`, `Figure`, `Table`, `Reference`
- `AbstractSection`, `Formula`, `Definition`, `TableCell`
- `JournalMeta`, `FundingInfo`, `HistoryDate`

Value objects should implement `PartialEq` and `Eq` based on content equality. They should be immutable once constructed.

**Entity vs Value Object Decision**:

```
Does the type have a meaningful, independent lifecycle outside the aggregate?
├─ Yes → Entity (has its own ID, can exist independently)
│        Example: PmcArticle (identified by PmcId)
└─ No  → Value Object (identified by its content)
         Example: Author (defined by name, not by database ID)

Is the type shared across aggregate boundaries?
├─ Yes → Place in common/ module
│        Example: Author, Affiliation, PmcId, PubMedId
└─ No  → Place in the bounded context module
         Example: Section (only in PMC), MeshHeading (only in PubMed)
```

### 5. DDD: Bounded Contexts

The workspace has two bounded contexts reflecting the underlying APIs:

| Context    | Concern            | Location                    |
| ---------- | ------------------ | --------------------------- |
| **PMC**    | Full-text articles | `pubmed-parser/src/pmc/`    |
| **PubMed** | Article metadata   | `pubmed-parser/src/pubmed/` |
| **Common** | Shared concepts    | `pubmed-parser/src/common/` |

**Context Mapping Rules**:

- Types that appear in both PMC and PubMed XMLs belong in `common/` (e.g., `Author`, ID types)
- Types specific to full-text structure belong in `pmc/` (e.g., `Section`, `Figure`)
- Types specific to metadata/search belong in `pubmed/` (e.g., `MeshHeading`, `ArticleType`)
- Never import from one bounded context into the other — use `common/` as the shared kernel

### 6. Ubiquitous Language

Use terminology from the biomedical literature domain, not software jargon:

| Use (Domain Term)    | Avoid (Tech Term)        |
| -------------------- | ------------------------ |
| `Article`, `Section` | `Document`, `Block`      |
| `Reference`          | `Link`, `Pointer`        |
| `Author`             | `User`, `Creator`        |
| `Figure`, `Table`    | `Image`, `Grid`          |
| `abstract_text`      | `summary`, `description` |
| `pub_dates`          | `timestamps`             |
| `pmcid`, `pmid`      | `id`, `external_id`      |
| `graphic_href`       | `image_url`, `file_path` |
| `supplement`         | `attachment`             |

## Design Decision Tree

```
Adding a new concept to the domain?
│
├─ Is it defined in a DTD/specification (JATS, NLM)?
│  ├─ Yes → Model it faithfully (see "DTD Faithfulness")
│  └─ No  → Is it derived/computed from DTD fields?
│           ├─ Yes → Add as a method on the domain type, NOT a field
│           └─ No  → It does NOT belong in the domain model
│
├─ Does it exist in both PubMed and PMC contexts?
│  ├─ Yes → Place in common/ module
│  └─ No  → Place in the appropriate bounded context (pmc/ or pubmed/)
│
├─ Is it an extraction/runtime artifact?
│  ├─ Yes → Place in pubmed-client crate, NOT pubmed-parser
│  └─ No  → Continue
│
├─ Does it require I/O, network, or file system access?
│  ├─ Yes → It belongs in pubmed-client, NOT pubmed-parser
│  └─ No  → Place in pubmed-parser domain model
│
└─ Could it conflict with existing public type names?
   ├─ Yes → Use a distinct name or namespace to avoid ambiguity
   └─ No  → Add directly
```

## Type Design Patterns

### Type-Safe Identifiers

Always use newtype wrappers for identifiers:

```rust
// ✅ Type-safe — compile-time guarantees, validated at parse boundary
pub pmcid: PmcId,
pub pmid: Option<PubMedId>,

// ❌ Raw strings — no validation, easy to mix up
pub pmcid: String,
pub pmid: Option<String>,
```

New ID types go in `pubmed-parser/src/common/ids.rs` and must implement:
`FromStr`, `Display`, `Serialize`, `Deserialize`, `Hash`, `PartialEq`, `Eq`, `Clone`, `Debug`

### Structured vs Flattened Data

Prefer structured types over flattened strings when the DTD provides structure:

```rust
// ✅ Structured — preserves DTD semantics, enables programmatic access
pub pub_dates: Vec<PublicationDate>,
pub struct PublicationDate {
    pub pub_type: Option<String>,  // "epub", "ppub"
    pub year: Option<u16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

// ❌ Flattened — loses structure, hard to query
pub pub_date: String,  // "2023-12-25"
```

### Enum vs String for Bounded Values

Use enums when the DTD defines a small, stable set of values. Use `String` when values are open-ended or vary across publishers:

```rust
// ✅ Enum — closed set with clear semantics
pub enum FormulaNotation {
    Tex,
    MathML,
    PlainText,
}

// ✅ String — section types are open-ended in practice
pub section_type: Option<String>,
```

### Recursive Structures

Sections in JATS nest arbitrarily. Model with self-referential types:

```rust
pub struct Section {
    pub title: Option<String>,
    pub content: String,
    pub subsections: Vec<Section>,  // Recursive
    pub figures: Vec<Figure>,
    pub tables: Vec<Table>,
}
```

### Separation of Extraction Concerns

Domain models describe **what the XML says**. Extraction results describe **what we did with it**:

```rust
// ✅ Domain model — only DTD fields (in pubmed-parser)
pub struct Figure {
    pub id: String,
    pub label: Option<String>,
    pub caption: String,
    pub graphic_href: Option<String>,   // From <graphic xlink:href="...">
}

// ✅ Extraction result — runtime artifact (in pubmed-client)
pub struct ExtractedFigure {
    pub figure: Figure,
    pub extracted_file_path: String,    // Where we saved it on disk
    pub file_size: Option<u64>,
    pub dimensions: Option<(u32, u32)>,
}
```

## Layered Architecture Rules

```
┌──────────────────────────────────────────────────────────┐
│  pubmed-client (async, HTTP, caching)                    │
│  - ExtractedFigure, OaSubsetInfo                         │
│  - Re-exports parser + domain types                      │
├──────────────────────────────────────────────────────────┤
│  pubmed-formatter (pure, formatting/export)               │
│  - PmcMarkdownConverter, ExportFormat                    │
│  - Consumes domain/parser models                         │
├──────────────────────────────────────────────────────────┤
│  pubmed-parser (pure, no I/O)                            │
│  - Domain models: PmcArticle (target)                    │
│  - Parser models: PmcFullText (to be replaced)           │
│  - Common: Author, PmcId, PubMedId                       │
└──────────────────────────────────────────────────────────┘
```

**Dependency rules** (never violated):

- `pubmed-parser` depends on NOTHING in the workspace
- `pubmed-formatter` depends only on `pubmed-parser`
- `pubmed-client` depends on both above
- Domain models NEVER depend upward (no client types in parser)
- Shared types live in `pubmed-parser/src/common/`

## Rich Domain Behavior

Domain models should not be anemic data bags. Add meaningful domain methods:

```rust
impl PmcArticle {
    /// Returns the earliest publication date.
    pub fn earliest_pub_date(&self) -> Option<&PublicationDate> { ... }

    /// Checks if the article has structured abstract sections.
    pub fn has_structured_abstract(&self) -> bool { ... }

    /// Returns all figures across all sections (flattened).
    pub fn all_figures(&self) -> Vec<&Figure> { ... }

    /// Returns sections of a given type (e.g., "methods").
    pub fn sections_by_type(&self, section_type: &str) -> Vec<&Section> { ... }
}
```

Methods should be **pure** — no I/O, no side effects, just queries and transformations on the aggregate's data.

## Text Mining Considerations

Since this toolkit serves text mining use cases, domain models should enable:

- **Section-level access**: Named section types (`intro`, `methods`, `results`, `discussion`) for targeted extraction
- **Structured abstracts**: `AbstractSection` with labels for section-aware NLP
- **Reference graphs**: `Reference` with cross-links (`pmid`, `doi`) for citation analysis
- **Table parsing**: `TableCell` with `colspan`/`rowspan` for structured data extraction
- **Formula representation**: `Formula` with notation type for math-aware processing
- **Glossary/definitions**: `Definition` for abbreviation resolution
- **Author metadata**: ORCID, roles, affiliations for author-level analysis

## Anti-Patterns to Avoid

### 1. Leaking Infrastructure Concerns

```rust
// ❌ Database concern in domain model
pub struct PmcArticle {
    pub id: i64,                     // DB auto-increment
    pub created_at: DateTime<Utc>,   // Persistence timestamp
}

// ✅ Pure domain — only DTD identity
pub struct PmcArticle {
    pub pmcid: PmcId,               // Domain identity from DTD
}
```

### 2. God Objects

```rust
// ❌ Flat — 50+ fields in one struct
pub struct Article {
    pub journal_title: String,
    pub journal_issn: String,
    pub journal_publisher: String,
    // ...
}

// ✅ Composed from value objects following DTD hierarchy
pub struct PmcArticle {
    pub journal: JournalMeta,        // <journal-meta>
    pub sections: Vec<Section>,      // <body><sec>...</sec></body>
}
```

### 3. Premature Generalization

```rust
// ❌ Over-abstracted
pub struct MetadataField<T> {
    pub value: T,
    pub source: String,
    pub confidence: f64,
}

// ✅ Specific and clear
pub struct PublicationDate {
    pub pub_type: Option<String>,
    pub year: Option<u16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}
```

### 4. Mixing Bounded Contexts

```rust
// ❌ PMC type importing PubMed-specific concepts
use crate::pubmed::MeshHeading;

pub struct PmcArticle {
    pub mesh_headings: Vec<MeshHeading>,  // PubMed concept in PMC context
}

// ✅ If shared, move to common/; if context-specific, keep separate
```

## Adding a New Domain Type — Step by Step

1. **Verify DTD source**: Check JATS tag library for the element
2. **Choose location**: `common/`, `pmc/domain.rs`, or `pubmed/domain.rs`
3. **Define the struct**: Map DTD elements to fields with documentation comments
4. **Add derives**: `Debug, Clone, Serialize, Deserialize, PartialEq`
5. **Add domain methods**: Pure queries/transformations relevant to text mining
6. **Implement `TryFrom`**: Convert from existing parser model if applicable
7. **Update module exports**: `mod.rs` in the appropriate module
8. **Add re-exports**: `pubmed-client/src/lib.rs` if public-facing
9. **Test**: `cargo test -p pubmed-parser` and `cargo test -p pubmed-client`
10. **Verify no breakage**: Existing tests still pass without modification

## Quick Reference: File Locations

| File                                     | Purpose                                     |
| ---------------------------------------- | ------------------------------------------- |
| `pubmed-parser/src/common/ids.rs`        | Type-safe identifiers (`PmcId`, `PubMedId`) |
| `pubmed-parser/src/common/models.rs`     | Shared types (`Author`, `Affiliation`)      |
| `pubmed-parser/src/pmc/domain.rs`        | PMC domain models (DTD-faithful)            |
| `pubmed-parser/src/pmc/parser/models.rs` | PMC parser models (being replaced)          |
| `pubmed-parser/src/pmc/parser/mod.rs`    | `parse_pmc_xml()`, `parse_pmc_xml_domain()` |
| `pubmed-parser/src/pubmed/models.rs`     | PubMed metadata models                      |
| `pubmed-client/src/lib.rs`               | Re-exports for public API                   |

## Success Indicators

After following this skill, verify:

- Every new field traces to a DTD element or attribute
- No I/O, persistence, or extraction artifacts in domain models
- Existing public APIs and tests remain unmodified and passing
- `TryFrom` conversions exist between parser and domain models
- Types are in the correct crate layer and bounded context
- Shared types are in `common/`, context-specific types in `pmc/` or `pubmed/`
- Domain types have meaningful methods, not just data fields
- `cargo test --workspace` passes
