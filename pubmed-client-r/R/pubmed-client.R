#' pubmedclient: PubMed and PMC API client
#'
#' R bindings for the Rust `pubmed-client` library. Search PubMed, fetch article
#' metadata, and retrieve PMC full text or Markdown.
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
