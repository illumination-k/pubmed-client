---
name: code-health
description: Investigate this workspace's code health with the agent-lens.toml profiles — find refactor targets, duplication, forwarding wrappers, complexity creep, or audit module structure. Use when the user asks "where should I refactor", "what are the hotspots/landmines", "find duplicates", "is anything too complex", "audit the architecture/coupling", or wants a code-quality sweep of the Rust crates.
---

This workspace ships an [`agent-lens.toml`](../../../agent-lens.toml) at the root with
tuned profiles. Always prefer `agent-lens run <profile>` over ad-hoc
`agent-lens analyze` calls — the profiles already set the right paths, exclude
fixtures/tmp/examples, and bundle the analyzers in a useful order. The binary is
provided by mise.

> The full quality sweep is large (similarity + wrapper + cohesion + complexity over
> the whole workspace is hundreds of lines). For anything beyond a single targeted
> profile, **delegate to the `code-health` subagent** (`Agent` tool, `subagent_type:
> "code-health"`) so the raw reports stay out of the main context and you get back a
> synthesized findings list.

## Profiles

| Profile    | Scope                                 | Analyzers                                 | Use for                                |
| ---------- | ------------------------------------- | ----------------------------------------- | -------------------------------------- |
| `hotspot`  | workspace (tests/fixtures excluded)   | hotspot                                   | Where churn × complexity meet          |
| `quality`  | workspace (tests/fixtures excluded)   | similarity, wrapper, cohesion, complexity | Full static-quality sweep              |
| `review`   | workspace, all `--diff-only`          | similarity, wrapper, cohesion, complexity | Pre-commit gate on unstaged changes    |
| `coupling` | `pubmed-client/src/lib.rs` crate root | coupling, context-span                    | Module layering / cycles / instability |

```bash
agent-lens run hotspot      # ranked refactor targets (start here)
agent-lens run quality      # full workspace report (markdown)
agent-lens run review       # only what the current diff touches
agent-lens run coupling     # one crate's module graph
```

`coupling` needs a single crate root, so it points at `pubmed-client/src/lib.rs`
(the crate that ties the workspace together). Swap `path` in `agent-lens.toml`, or run
`agent-lens analyze coupling pubmed-parser/src/lib.rs --format md` to inspect another
crate.

## Investigation workflow

1. **Start with `hotspot`.** It ranks `commits × cognitive_max` — "frequently changed
   _and_ complex". The top ~5 files are where bugs concentrate and refactors pay off.
2. **Drill into `complexity`** for those files. The per-function list gives exact line
   ranges (`file:Func (L238-317): cog=50`). Sort by **cognitive**, not cyclomatic.
3. **Cross-check `similarity` + `wrapper`** to see whether the complexity is genuine
   logic or copy-paste / thin forwarding that should be unified.
4. **For structure questions, run `coupling`.** Read `instability` (`Ce/(Ca+Ce)`),
   Fan-In/Fan-Out, IFC, and cycle count. `context-span` estimates onboarding cost (how
   many files you must open to reason about a module).
5. **Before committing, run `review`** to catch complexity/cohesion/duplication
   regressions the current diff introduces. An empty report is the success case.
6. **Report findings, then confirm before editing.** These analyzers measure
   _shape, not semantics_ — every flagged item needs a human judgment call (see below).
   Cite `file:line` from the report so the user can jump straight there.

## Reading the output — repo-specific signal vs. noise

This is a layered Rust workspace (`pubmed-parser` → `pubmed-formatter` →
`pubmed-client` → bindings/cli/mcp). Several large clusters are **by-design
boilerplate** — learn to skip them so the real findings stand out.

### Mostly noise (intentional by design)

- **Language-binding glue dominates `similarity`.** The biggest clusters live in
  `pubmed-client-py/`, `pubmed-client-napi/src/lib.rs`, and
  `pubmed-client-wasm/src/lib.rs` — the `*::from` DTO mappers (`Summary`/`JsSummary`/
  `PyArticleSummary`) and the `fetch_articles` / `fetch_summaries` / `get_*` methods
  that each forward to the same core-crate call. These are **intentional per-language
  surfaces** built with different macros (PyO3 / napi / wasm-bindgen); they can't share
  an implementation. Don't churn them. To focus a sweep on the Rust core, run
  `agent-lens analyze similarity pubmed-parser pubmed-formatter pubmed-client --format md`
  (or `--exclude 'pubmed-client-py/**' --exclude 'pubmed-client-napi/**' --exclude 'pubmed-client-wasm/**'`).
- **`Client::*` wrappers in `pubmed-client/src/lib.rs`** (12 methods forwarding to
  `self.pubmed.*`) are the **unified-client facade** documented in CLAUDE.md ("Unified
  client with `pubmed` and `pmc` fields; convenience methods"). Public API, not a smell.
- **`PmcArticle::doi`/`title`/`volume`/… in `pmc/domain.rs`** forwarding to nested
  `front.article_meta.*` are the **flat accessor layer** CLAUDE.md describes ("Single
  PMC model layer; flat read access via accessor methods"). Intentional.
- **Builder-setter cohesion** — `WasmClientConfig` (LCOM4=5), `ClientConfig` (LCOM4=6),
  `RetryConfig` — each `with_*` setter touches a different field, so a high LCOM4 is the
  expected shape of a builder, not a defect.
- **`SearchQuery::published_between` / `entry_date_between` / `modification_date_between`**
  (≈98% similar) and `and`/`or`, `negate`/`group` in `query/boolean.rs` are
  builder-pattern variants. A shared private helper is _possible_ but low-value/low-risk.
- **`examples/**` and `benches/**` duplication** (`build_batch_xml`, `display_article`,
  `load_all_*_xmls`) is demo/bench code kept standalone for readability.

### Real, actionable signal

- **`pubmed-parser/src/pmc/parser/` is the complexity epicentre.**
  `author.rs:extract_reference_authors` (cog=50, the workspace max),
  `section.rs:parse_section_from_body` (cog=39) and `extract_body_sections` (cog=29),
  `xml_utils.rs:decode_xml_entities` (cog=37). `section.rs`, `metadata.rs`, `author.rs`
  are also top hotspots. These are genuine parsing logic — split/simplify candidates.
- **`pubmed-client/src/pmc/tar.rs` is the #1 hotspot** (score 340, heavy churn +
  `find_matching_file` cog=34). Watch this file on every change.
- **Genuine accidental duplicates** worth unifying (same logic, _within the Rust core_):
  - `format_first_pub_date` is **100% duplicated** across
    `pubmed-formatter/src/pmc/markdown/frontmatter.rs` and `.../markdown/metadata.rs`.
  - `format_author_name` is ~94% duplicated across `pubmed-parser/src/common/models.rs`
    and `pubmed-parser/src/pubmed/parser/extractors.rs`.
  - `PubMedId::try_from_u32` / `PmcId::try_from_u32` in `common/ids.rs` and
    `PmcXmlTestCase::new` / `PubMedXmlTestCase::new` in `pubmed-test-utils` are
    type-paired — macro candidates if the pattern grows.
- **`wrapper` to shared helpers** is actionable when a thin per-module wrapper exists
  only to forward to a canonical helper (e.g. `PmcClient::normalize_pmcid` →
  `common::normalize_pmcid`). The repo prefers one unified helper over parallel variants.
- **`coupling` on `pubmed-client`**: `crate::error` is a high-Fan-In/low-Fan-Out stable
  hub (healthy); `pmc::client` / `pmc::tar` are intentionally high-instability leaves.
  The signal to watch is **cycle count > 0** (currently 0) — flag any new cross-crate
  cycle or new dependency that inverts the `parser → formatter → client` layering.

## Tuning

Adjust thresholds in [`agent-lens.toml`](../../../agent-lens.toml), or override
per-invocation with `agent-lens analyze <tool> <path> --format md` flags
(`--threshold`, `--min-lines`, `--top`, `--diff-only`, `--since`, `--exclude`). Full
reference: `agent-lens help --md`.
