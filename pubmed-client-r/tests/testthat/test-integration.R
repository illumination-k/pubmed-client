# Integration tests hit the live NCBI E-utilities API. They are opt-in to keep
# CI and CRAN deterministic: set PUBMED_REAL_API_TESTS=1 to run them (mirrors the
# Rust crate's PUBMED_REAL_API_TESTS gate).

skip_unless_real_api <- function() {
  if (!nzchar(Sys.getenv("PUBMED_REAL_API_TESTS"))) {
    testthat::skip("set PUBMED_REAL_API_TESTS=1 to run live API tests")
  }
  testthat::skip_if_offline()
}

test_that("pubmed_search returns PMIDs", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  ids <- pubmed_search(client, "crispr", limit = 3)
  expect_type(ids, "character")
  expect_lte(length(ids), 3)
})

test_that("pubmed_fetch returns article metadata", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  article <- pubmed_fetch(client, "31978945")
  expect_type(article, "list")
  expect_equal(article$pmid, "31978945")
  expect_true(nzchar(article$title))
})

test_that("pmc_to_markdown renders Markdown", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  md <- pmc_to_markdown(client, "PMC7906746")
  expect_type(md, "character")
  expect_true(nzchar(md))
})

# Europe PMC needs no API key, but these still hit the live EBI service, so they
# sit behind the same opt-in gate.

test_that("europepmc_search returns records", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  results <- europepmc_search(client, "malaria vaccine", limit = 3)
  expect_type(results, "list")
  expect_gt(length(results), 0)
  first <- results[[1]]
  expect_true(nzchar(first$id))
  expect_equal(first$europe_pmc_id, paste0(first$source, "/", first$id))
})

test_that("europepmc_search reaches preprints", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  results <- europepmc_search(client, "SRC:PPR AND TITLE:CRISPR", limit = 3)
  for (result in results) {
    expect_equal(result$source, "PPR")
  }
})

test_that("europepmc_fulltext returns article metadata", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  article <- europepmc_fulltext(client, "PMC3258128")
  expect_equal(article$pmcid, "PMC3258128")
  expect_true(nzchar(article$title))
})

test_that("europepmc_fulltext_xml returns JATS", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  xml <- europepmc_fulltext_xml(client, "PMC3258128")
  expect_type(xml, "character")
  expect_true(grepl("<article", xml, fixed = TRUE))
})

test_that("europepmc_references and europepmc_citations return the graph", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  references <- europepmc_references(client, "PMC3258128")
  expect_gt(length(references), 0)
  citations <- europepmc_citations(client, "PMC3258128")
  expect_gt(length(citations), 0)
})

test_that("europepmc_database_links returns cross-reference groups", {
  skip_unless_real_api()
  client <- pubmed_client(email = "ci@example.com")
  # A record may legitimately have none, so assert on shape, not presence.
  links <- europepmc_database_links(client, "PMC3258128")
  expect_type(links, "list")
  for (link in links) {
    expect_true(nzchar(link$db_name))
    expect_type(link$info, "list")
  }
})
