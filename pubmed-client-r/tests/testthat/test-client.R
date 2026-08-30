# Offline tests: client construction and input validation. These do not touch
# the network (client_new only builds configuration).

test_that("pubmed_client() returns a pubmed_client object", {
  client <- pubmed_client()
  expect_s3_class(client, "pubmed_client")
})

test_that("pubmed_client() accepts configuration arguments", {
  client <- pubmed_client(
    api_key = "dummy",
    email = "you@example.com",
    tool = "pubmedclient-tests",
    rate_limit = 5,
    timeout_seconds = 10
  )
  expect_s3_class(client, "pubmed_client")
})

test_that("print method is silent-friendly", {
  client <- pubmed_client()
  expect_output(print(client), "pubmed_client")
})

test_that("API functions reject non-client input", {
  expect_error(pubmed_search("not a client", "x"), "pubmed_client")
  expect_error(pubmed_fetch(list(), "1"), "pubmed_client")
  expect_error(pubmed_search_and_fetch(NULL, "x"), "pubmed_client")
  expect_error(pmc_fulltext(42, "PMC1"), "pubmed_client")
  expect_error(pmc_to_markdown(NA, "PMC1"), "pubmed_client")
})

test_that("Europe PMC functions reject non-client input", {
  expect_error(europepmc_search("not a client", "x"), "pubmed_client")
  expect_error(europepmc_fulltext(list(), "PMC1"), "pubmed_client")
  expect_error(europepmc_fulltext_xml(NULL, "PMC1"), "pubmed_client")
  expect_error(europepmc_references(42, "PMC1"), "pubmed_client")
  expect_error(europepmc_citations(NA, "PMC1"), "pubmed_client")
  expect_error(europepmc_database_links("", "PMC1"), "pubmed_client")
})

# A Europe PMC record address is validated before any request is issued, so
# these stay offline. They assert only that an error is raised: extendr is built
# without its `result_condition` feature, so every error the Rust side returns
# reaches R as "User function panicked: <fn>" rather than carrying its own
# message. That applies to the PubMed and PMC functions too and is not specific
# to Europe PMC.

test_that("Europe PMC rejects an invalid record address", {
  client <- pubmed_client()
  expect_error(europepmc_references(client, "   "))
  expect_error(europepmc_citations(client, "MED/"))
  expect_error(europepmc_fulltext(client, "not-a-pmcid", source = "PMC"))
})

test_that("Europe PMC rejects an unknown result type", {
  client <- pubmed_client()
  expect_error(europepmc_search(client, "cancer", limit = 1, result_type = "verbose"))
})
