/*
 * C ABI for the PubMed / PMC client, implemented by the Rust crate in
 * ../rust and consumed by the Go package in the parent directory.
 *
 * Ownership rules
 * ---------------
 *  - Every `char *` returned by a call function, and every message written to
 *    an `out_err`, is owned by the caller and must be released with
 *    `pubmed_string_free`.
 *  - A NULL return means failure; `*out_err` then holds the message. On
 *    success `*out_err` is NULL.
 *  - `pubmed_client_version` returns a static string that must NOT be freed.
 *  - Handles from `pubmed_client_new` are released with `pubmed_client_free`,
 *    and tokens from `pubmed_cancel_new` with `pubmed_cancel_free`.
 *
 * Error envelope
 * --------------
 * `*out_err` holds a JSON object:
 *
 *     {"kind": "article_not_found", "message": "...", "status": 404}
 *
 * `kind` classifies the failure (see `ErrorKind` in ../rust/src/error.rs) and
 * `status` is present only for `"api"`. A caller that cannot parse the
 * envelope should treat the whole string as the message.
 *
 * Cancellation
 * ------------
 * Every call function takes a `cancel` token, which may be NULL. A non-NULL
 * token can be fired from another thread with `pubmed_cancel_trigger` to abort
 * the call, which then fails with kind `"cancelled"`. The token must outlive
 * the call.
 *
 * Unless noted otherwise, results are UTF-8 JSON. Calls block until the
 * underlying HTTP request completes.
 */

#ifndef PUBMED_CLIENT_H
#define PUBMED_CLIENT_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque client handle. */
typedef struct PubmedClient PubmedClient;

/* Opaque cancellation token. */
typedef struct PubmedCancel PubmedCancel;

/* ---------------------------------------------------------------------------
 * Lifecycle
 * ------------------------------------------------------------------------- */

/* Version of the underlying Rust crate. Statically allocated; do not free. */
const char *pubmed_client_version(void);

/*
 * Create a client. `config_json` may be NULL for library defaults, otherwise a
 * JSON object with any of: api_key, email, tool (strings), rate_limit (number),
 * timeout_seconds (integer), user_agent, base_url (strings), cache (bool).
 * Unknown keys are rejected.
 */
PubmedClient *pubmed_client_new(const char *config_json, char **out_err);

/* Release a handle from pubmed_client_new. NULL is a no-op. */
void pubmed_client_free(PubmedClient *client);

/* Release a string produced by this library. NULL is a no-op. */
void pubmed_string_free(char *value);

/* ---------------------------------------------------------------------------
 * Cancellation
 * ------------------------------------------------------------------------- */

/* Create a cancellation token. Never returns NULL. */
PubmedCancel *pubmed_cancel_new(void);

/*
 * Fire a token, aborting any call currently using it. Safe to call from any
 * thread, more than once, or after the call has already returned. NULL is a
 * no-op.
 */
void pubmed_cancel_trigger(const PubmedCancel *cancel);

/*
 * Release a token from pubmed_cancel_new. NULL is a no-op. Must not be called
 * while a call is still using the token.
 */
void pubmed_cancel_free(PubmedCancel *cancel);

/* ---------------------------------------------------------------------------
 * PubMed: search and metadata
 * ------------------------------------------------------------------------- */

/*
 * Search PubMed. Returns a JSON array of PMID strings. `sort` may be NULL for
 * PubMed's default ordering, otherwise one of "relevance", "pub_date",
 * "author", "journal".
 */
char *pubmed_search_articles(const PubmedClient *client, const char *query,
                             size_t limit, const char *sort,
                             const PubmedCancel *cancel, char **out_err);

/* Fetch metadata for one PMID. Returns a JSON article object. */
char *pubmed_fetch_article(const PubmedClient *client, const char *pmid,
                           const PubmedCancel *cancel, char **out_err);

/* Fetch metadata for a JSON array of PMID strings. Returns a JSON array. */
char *pubmed_fetch_articles(const PubmedClient *client, const char *pmids_json,
                            const PubmedCancel *cancel, char **out_err);

/*
 * Fetch metadata for an arbitrarily large JSON array of PMID strings by way of
 * the history server. Returns a JSON array of articles.
 */
char *pubmed_fetch_all_by_pmids(const PubmedClient *client,
                                const char *pmids_json,
                                const PubmedCancel *cancel, char **out_err);

/* Search and fetch metadata in one call. Returns a JSON array of articles. */
char *pubmed_search_and_fetch(const PubmedClient *client, const char *query,
                              size_t limit, const char *sort,
                              const PubmedCancel *cancel, char **out_err);

/*
 * Search and attach PMC full text where available. Returns a JSON array of
 * {"article": ..., "full_text": ...|null} objects.
 */
char *pubmed_search_with_full_text(const PubmedClient *client,
                                   const char *query, size_t limit,
                                   const PubmedCancel *cancel, char **out_err);

/* ---------------------------------------------------------------------------
 * PubMed: ESummary
 * ------------------------------------------------------------------------- */

/* Fetch summaries for a JSON array of PMID strings. Returns a JSON array. */
char *pubmed_fetch_summaries(const PubmedClient *client, const char *pmids_json,
                             const PubmedCancel *cancel, char **out_err);

/* Search and fetch a summary per hit. Returns a JSON array of summaries. */
char *pubmed_search_and_fetch_summaries(const PubmedClient *client,
                                        const char *query, size_t limit,
                                        const char *sort,
                                        const PubmedCancel *cancel,
                                        char **out_err);

/* ---------------------------------------------------------------------------
 * PubMed: ELink, EInfo, EGQuery, ECitMatch, ESpell
 * ------------------------------------------------------------------------- */

/*
 * The ELink calls take a JSON array of numeric PMIDs (not strings) and return
 * a JSON object naming both the sources and what was found.
 */
char *pubmed_get_related_articles(const PubmedClient *client,
                                  const char *pmids_json,
                                  const PubmedCancel *cancel, char **out_err);
char *pubmed_get_pmc_links(const PubmedClient *client, const char *pmids_json,
                           const PubmedCancel *cancel, char **out_err);
char *pubmed_get_citations(const PubmedClient *client, const char *pmids_json,
                           const PubmedCancel *cancel, char **out_err);

/* List available NCBI databases. Returns a JSON array of names. */
char *pubmed_get_database_list(const PubmedClient *client,
                               const PubmedCancel *cancel, char **out_err);

/* Describe one NCBI database. Returns a JSON DatabaseInfo object. */
char *pubmed_get_database_info(const PubmedClient *client,
                               const char *database,
                               const PubmedCancel *cancel, char **out_err);

/* Count matches across every Entrez database. Returns a JSON object. */
char *pubmed_global_query(const PubmedClient *client, const char *term,
                          const PubmedCancel *cancel, char **out_err);

/*
 * Resolve citations to PMIDs. `citations_json` is a JSON array of
 * {journal, year, volume, first_page, author_name, key} objects.
 */
char *pubmed_match_citations(const PubmedClient *client,
                             const char *citations_json,
                             const PubmedCancel *cancel, char **out_err);

/*
 * Spell-check a search term. `database` may be NULL for PubMed. Returns a JSON
 * SpellCheckResult object.
 */
char *pubmed_spell_check(const PubmedClient *client, const char *term,
                         const char *database, const PubmedCancel *cancel,
                         char **out_err);

/* ---------------------------------------------------------------------------
 * PMC
 * ------------------------------------------------------------------------- */

/* Fetch PMC full text. Returns a JSON article object. */
char *pmc_fetch_full_text(const PubmedClient *client, const char *pmcid,
                          const PubmedCancel *cancel, char **out_err);

/* Fetch the raw JATS XML. Returns the XML, not JSON. */
char *pmc_fetch_xml(const PubmedClient *client, const char *pmcid,
                    const PubmedCancel *cancel, char **out_err);

/*
 * Fetch PMC full text rendered to Markdown. Returns Markdown, not JSON.
 * `options_json` may be NULL for the default rendering (see
 * `MarkdownOptionsDto` in ../rust/src/pmc.rs).
 */
char *pmc_fetch_markdown(const PubmedClient *client, const char *pmcid,
                         const char *options_json, const PubmedCancel *cancel,
                         char **out_err);

/*
 * Check whether a PMID has PMC full text. Returns a JSON string holding the
 * PMCID, or JSON `null` when unavailable.
 */
char *pmc_check_availability(const PubmedClient *client, const char *pmid,
                             const PubmedCancel *cancel, char **out_err);

/* Report Open Access subset status. Returns a JSON OaSubsetInfo object. */
char *pmc_is_oa_subset(const PubmedClient *client, const char *pmcid,
                       const PubmedCancel *cancel, char **out_err);

/* Download an OA article's files. Returns a JSON array of written paths. */
char *pmc_download_files(const PubmedClient *client, const char *pmcid,
                         const char *output_dir, const PubmedCancel *cancel,
                         char **out_err);

/*
 * Download an OA article's figures and pair each with its caption. Returns a
 * JSON array of ExtractedFigure objects.
 */
char *pmc_extract_figures(const PubmedClient *client, const char *pmcid,
                          const char *output_dir, const PubmedCancel *cancel,
                          char **out_err);

/* Drop every cached PMC response. Returns JSON `null`. */
char *pmc_clear_cache(const PubmedClient *client, const PubmedCancel *cancel,
                      char **out_err);

/* ---------------------------------------------------------------------------
 * Europe PMC
 *
 * Europe PMC records are addressed by a source database plus an id. Every call
 * below takes both; `source` may be NULL, in which case a "PMC"-prefixed id is
 * read as a PMC record and anything else as a PubMed (MED) record. An id given
 * in fully-qualified "SOURCE/ID" form wins over `source`.
 *
 * `options_json` on the search calls may be NULL for the defaults, otherwise
 * {"result_type": "idlist"|"lite"|"core", "page_size": number,
 *  "cursor_mark": string, "sort": string} (see `SearchOptionsDto` in
 * ../rust/src/europe_pmc.rs).
 * ------------------------------------------------------------------------- */

/*
 * Search across pages until `limit` records are collected. Returns a JSON array
 * of Europe PMC records.
 */
char *europe_pmc_search(const PubmedClient *client, const char *query,
                        size_t limit, const char *options_json,
                        const PubmedCancel *cancel, char **out_err);

/*
 * Fetch one page of search results. Returns a JSON object with the total hit
 * count, the next cursor, and the page's records.
 */
char *europe_pmc_search_page(const PubmedClient *client, const char *query,
                             const char *options_json,
                             const PubmedCancel *cancel, char **out_err);

/*
 * Fetch and parse full text. Returns a JSON article object. Supports
 * PMC-sourced records only; use `europe_pmc_fetch_xml` for other sources.
 */
char *europe_pmc_fetch_full_text(const PubmedClient *client, const char *id,
                                 const char *source,
                                 const PubmedCancel *cancel, char **out_err);

/* Fetch the raw JATS XML for a record. Returns the XML, not JSON. */
char *europe_pmc_fetch_xml(const PubmedClient *client, const char *id,
                           const char *source, const PubmedCancel *cancel,
                           char **out_err);

/* Fetch every work cited by a record. Returns a JSON array. */
char *europe_pmc_get_references(const PubmedClient *client, const char *id,
                                const char *source, const PubmedCancel *cancel,
                                char **out_err);

/* Fetch every article citing a record. Returns a JSON array. */
char *europe_pmc_get_citations(const PubmedClient *client, const char *id,
                               const char *source, const PubmedCancel *cancel,
                               char **out_err);

/* Fetch external database cross-references. Returns a JSON array. */
char *europe_pmc_get_database_links(const PubmedClient *client, const char *id,
                                    const char *source,
                                    const PubmedCancel *cancel,
                                    char **out_err);

/*
 * Download a record's supplementary-files ZIP to `output_path`. Returns the
 * written path as a JSON string.
 */
char *europe_pmc_download_supplementary_files(
    const PubmedClient *client, const char *id, const char *source,
    const char *output_path, const PubmedCancel *cancel, char **out_err);

/* ---------------------------------------------------------------------------
 * Pure functions (no client, no network)
 * ------------------------------------------------------------------------- */

/*
 * Build a PubMed query string from a recorded builder operation list.
 * `request_json` is {"ops": [...], "validate": bool}; the result is
 * {"query": string, "limit": number, "sort": string|null}.
 */
char *pubmed_query_build(const char *request_json, char **out_err);

/*
 * Export a JSON array of articles as a citation document. `format` is one of
 * "bibtex", "ris", "csl-json", "nbib". Returns the rendered document.
 */
char *pubmed_export_articles(const char *articles_json, const char *format,
                             char **out_err);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* PUBMED_CLIENT_H */
