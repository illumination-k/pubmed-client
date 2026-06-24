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
