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
 *  - Handles from `pubmed_client_new` are released with `pubmed_client_free`.
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

/* Search PubMed. Returns a JSON array of PMID strings. */
char *pubmed_search_articles(const PubmedClient *client, const char *query,
                             size_t limit, char **out_err);

/* Fetch metadata for one PMID. Returns a JSON article object. */
char *pubmed_fetch_article(const PubmedClient *client, const char *pmid,
                           char **out_err);

/* Fetch metadata for a JSON array of PMID strings. Returns a JSON array. */
char *pubmed_fetch_articles(const PubmedClient *client, const char *pmids_json,
                            char **out_err);

/* Search and fetch metadata in one call. Returns a JSON array of articles. */
char *pubmed_search_and_fetch(const PubmedClient *client, const char *query,
                              size_t limit, char **out_err);

/* Fetch PMC full text. Returns a JSON article object. */
char *pmc_fetch_full_text(const PubmedClient *client, const char *pmcid,
                          char **out_err);

/* Fetch PMC full text rendered to Markdown. Returns Markdown, not JSON. */
char *pmc_fetch_markdown(const PubmedClient *client, const char *pmcid,
                         char **out_err);

/*
 * Check whether a PMID has PMC full text. Returns a JSON string holding the
 * PMCID, or JSON `null` when unavailable.
 */
char *pmc_check_availability(const PubmedClient *client, const char *pmid,
                             char **out_err);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* PUBMED_CLIENT_H */
