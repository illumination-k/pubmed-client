#' pubmedclient: PubMed and PMC API client
#'
#' R bindings for the Rust `pubmed-client` library. Search PubMed, fetch article
#' metadata, retrieve PMC full text or Markdown, and reach Europe PMC for
#' cross-source search and citation graphs.
#'
#' @keywords internal
#' @useDynLib pubmedclient, .registration = TRUE
"_PACKAGE"

#' Create a PubMed/PMC client
#'
#' All arguments are optional. Supplying an NCBI `api_key` raises the rate limit
#' from 3 to 10 requests per second; `email` and `tool` are recommended by NCBI
#' for identification.
#'
#' @param api_key Optional NCBI API key.
#' @param email Optional contact email used to identify requests.
#' @param tool Optional tool name (defaults to the library default).
#' @param rate_limit Optional requests-per-second override.
#' @param timeout_seconds Optional HTTP request timeout in seconds.
#'
#' @return A `pubmed_client` object.
#' @export
#'
#' @examples
#' \dontrun{
#' client <- pubmed_client(email = "you@example.com")
#' ids <- pubmed_search(client, "crispr", limit = 5)
#' }
pubmed_client <- function(api_key = NULL,
                          email = NULL,
                          tool = NULL,
                          rate_limit = NULL,
                          timeout_seconds = NULL) {
  ptr <- client_new(
    api_key,
    email,
    tool,
    if (is.null(rate_limit)) NULL else as.numeric(rate_limit),
    if (is.null(timeout_seconds)) NULL else as.numeric(timeout_seconds)
  )
  structure(list(ptr = ptr), class = "pubmed_client")
}

#' @export
print.pubmed_client <- function(x, ...) {
  cat("<pubmed_client>\n")
  invisible(x)
}

# Fail early with a clear message if a non-client is passed.
.check_client <- function(client) {
  if (!inherits(client, "pubmed_client")) {
    stop("`client` must be created with `pubmed_client()`", call. = FALSE)
  }
}

#' Search PubMed
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param query PubMed search query string.
#' @param limit Maximum number of PMIDs to return.
#'
#' @return A character vector of PMIDs.
#' @export
pubmed_search <- function(client, query, limit = 20L) {
  .check_client(client)
  client_search_articles(client$ptr, query, as.integer(limit))
}

#' Fetch article metadata
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param pmids One or more PMIDs.
#'
#' @return For a single PMID, a named list of article fields. For several PMIDs,
#'   a list of such named lists.
#' @export
pubmed_fetch <- function(client, pmids) {
  .check_client(client)
  pmids <- as.character(pmids)
  if (length(pmids) == 1L) {
    client_fetch_article(client$ptr, pmids)
  } else {
    client_fetch_articles(client$ptr, pmids)
  }
}

#' Search PubMed and fetch metadata in one call
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param query PubMed search query string.
#' @param limit Maximum number of articles to fetch.
#'
#' @return A list of named lists, one per article.
#' @export
pubmed_search_and_fetch <- function(client, query, limit = 20L) {
  .check_client(client)
  client_search_and_fetch(client$ptr, query, as.integer(limit))
}

#' Fetch PMC full-text summary metadata
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param pmcid A PMC identifier, e.g. `"PMC7906746"`.
#'
#' @return A named list with `pmcid`, `pmid`, `title`, `doi`, and section,
#'   author, and reference counts.
#' @export
pmc_fulltext <- function(client, pmcid) {
  .check_client(client)
  pmc_fetch_fulltext(client$ptr, pmcid)
}

#' Fetch a PMC article rendered as Markdown
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param pmcid A PMC identifier, e.g. `"PMC7906746"`.
#'
#' @return A length-one character vector containing the Markdown document.
#' @export
pmc_to_markdown <- function(client, pmcid) {
  .check_client(client)
  pmc_markdown(client$ptr, pmcid)
}

# ------------------------------------------------------------------------------
# Europe PMC
#
# Europe PMC (https://europepmc.org) is a complementary index to the NCBI
# E-utilities: it covers preprints (PPR), patents (PAT), Agricola (AGR) and
# Chinese Biological Abstracts (CBA) as well as PubMed (MED) and PMC, and needs
# no API key.
#
# Records are addressed by a source database plus an id. The functions below
# take both; `source` may be NULL, in which case a "PMC"-prefixed id is read as
# a PMC record and anything else as a PubMed record. An id given in
# fully-qualified "SOURCE/ID" form wins over `source`.
# ------------------------------------------------------------------------------

#' Search Europe PMC
#'
#' Searches every source Europe PMC indexes, following pages until `limit`
#' records are collected or the result set is exhausted. The query uses Europe
#' PMC's own syntax, e.g. `"TITLE:CRISPR AND SRC:PPR"`.
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param query Europe PMC search query string.
#' @param limit Maximum number of records to return.
#' @param result_type Level of detail: `"idlist"`, `"lite"` (the default), or
#'   `"core"`. `"core"` returns far more fields than are modelled; the
#'   remainder is available in each record's `extra_json`.
#' @param sort Optional Europe PMC sort expression, e.g. `"P_PDATE_D desc"` for
#'   newest first or `"CITED desc"` for most cited.
#'
#' @return A list of named lists, one per record, each with `id`, `source`,
#'   `europe_pmc_id`, `pmid`, `pmcid`, `doi`, `title`, `author_string`,
#'   `journal_title`, `pub_year`, `is_open_access`, and `extra_json`.
#' @export
#'
#' @examples
#' \dontrun{
#' client <- pubmed_client(email = "you@example.com")
#' preprints <- europepmc_search(client, "TITLE:CRISPR AND SRC:PPR", limit = 5)
#' }
europepmc_search <- function(client, query, limit = 20L, result_type = NULL, sort = NULL) {
  .check_client(client)
  epmc_search(
    client$ptr,
    query,
    as.integer(limit),
    if (is.null(result_type)) NULL else as.character(result_type),
    if (is.null(sort)) NULL else as.character(sort)
  )
}

#' Fetch Europe PMC full-text summary metadata
#'
#' Parsing into an article requires a PMC identifier, so this supports
#' PMC-sourced records only; use [europepmc_fulltext_xml()] for other sources.
#'
#' @param client A `pubmed_client` created by [pubmed_client()].
#' @param id A record id, bare (`"PMC3258128"`, `"33515491"`) or fully
#'   qualified (`"PPR/PPR123456"`).
#' @param source Optional source database: `"MED"`, `"PMC"`, `"PPR"`, `"AGR"`,
#'   `"CBA"`, `"PAT"`.
#'
#' @return A named list with `pmcid`, `pmid`, `title`, `doi`, and section,
#'   author, and reference counts.
#' @export
europepmc_fulltext <- function(client, id, source = NULL) {
  .check_client(client)
  epmc_fetch_fulltext(client$ptr, id, .epmc_source(source))
}

#' Fetch the raw JATS XML for a Europe PMC record
#'
#' Works for any source that has full text available, including those
#' [europepmc_fulltext()] cannot parse into an article.
#'
#' @inheritParams europepmc_fulltext
#'
#' @return A length-one character vector containing the JATS XML.
#' @export
europepmc_fulltext_xml <- function(client, id, source = NULL) {
  .check_client(client)
  epmc_fetch_xml(client$ptr, id, .epmc_source(source))
}

#' List the works a Europe PMC record cites
#'
#' @inheritParams europepmc_fulltext
#'
#' @return A list of named lists, one per cited work. `source` and `id` are
#'   `NULL` for a reference Europe PMC could not match to one of its records.
#' @export
europepmc_references <- function(client, id, source = NULL) {
  .check_client(client)
  epmc_references(client$ptr, id, .epmc_source(source))
}

#' List the articles citing a Europe PMC record
#'
#' Coverage is broader than PubMed's own citation links: preprints and other
#' non-PubMed sources are included.
#'
#' @inheritParams europepmc_fulltext
#'
#' @return A list of named lists, one per citing article, each including that
#'   article's own `cited_by_count`.
#' @export
europepmc_citations <- function(client, id, source = NULL) {
  .check_client(client)
  epmc_citations(client$ptr, id, .epmc_source(source))
}

#' List a Europe PMC record's external database cross-references
#'
#' Europe PMC links records to external biological databases (UniProt, EMBL,
#' PDB, ChEBI, ArrayExpress, ...). A record with no cross-references returns an
#' empty list rather than an error.
#'
#' @inheritParams europepmc_fulltext
#'
#' @return A list of named lists, one per external database, each with
#'   `db_name`, `db_count`, and an `info` list of cross-reference entries.
#'   Europe PMC documents the four `info` slots only positionally, so they are
#'   returned as `info1` to `info4` rather than renamed.
#' @export
europepmc_database_links <- function(client, id, source = NULL) {
  .check_client(client)
  epmc_database_links(client$ptr, id, .epmc_source(source))
}

# Normalise the optional source argument to NULL or a length-one character.
.epmc_source <- function(source) {
  if (is.null(source)) NULL else as.character(source)
}
