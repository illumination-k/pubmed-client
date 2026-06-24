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
