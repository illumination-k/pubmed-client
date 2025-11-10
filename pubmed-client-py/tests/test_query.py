"""Unit tests for SearchQuery Python bindings."""

import pytest


def test_searchquery_constructor_creates_empty_query() -> None:
    """Test that SearchQuery() creates an empty query that raises ValueError on build()."""
    from pubmed_client import SearchQuery

    query = SearchQuery()
    # Empty query should raise ValueError when build() is called
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_query_single_term() -> None:
    """Test adding a single search term."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("covid-19")
    assert query.build() == "covid-19"


def test_query_multiple_calls_accumulate() -> None:
    """Test that multiple query() calls accumulate terms."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("covid-19").query("treatment")
    assert query.build() == "covid-19 treatment"


def test_terms_batch_addition() -> None:
    """Test adding multiple terms at once via terms() method."""
    from pubmed_client import SearchQuery

    query = SearchQuery().terms(["covid-19", "vaccine", "efficacy"])
    assert query.build() == "covid-19 vaccine efficacy"


def test_query_none_filtered_silently() -> None:
    """Test that None values in query() are silently filtered."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query(None).query("covid-19")
    assert query.build() == "covid-19"


def test_terms_none_filtered_silently() -> None:
    """Test that None values in terms() list are silently filtered."""
    from pubmed_client import SearchQuery

    terms: list[str | None] = [None, "covid-19", None, "vaccine"]
    query = SearchQuery().terms(terms)
    assert query.build() == "covid-19 vaccine"


def test_query_empty_string_filtered() -> None:
    """Test that empty strings and whitespace-only strings are filtered."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("").query("   ").query("cancer")
    assert query.build() == "cancer"


def test_limit_valid_values() -> None:
    """Test that valid limit values are accepted."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("cancer").limit(50)
    # Limit doesn't appear in build() output (used during execution)
    assert query.build() == "cancer"

    # Test boundary values
    query_min = SearchQuery().query("cancer").limit(1)
    assert query_min.build() == "cancer"

    query_max = SearchQuery().query("cancer").limit(10000)
    assert query_max.build() == "cancer"


def test_limit_none_uses_default() -> None:
    """Test that limit(None) is treated as unset (uses default of 20)."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("cancer").limit(None)
    # Should not raise error, None means "use default"
    assert query.build() == "cancer"


def test_limit_zero_raises_valueerror() -> None:
    """Test that limit(0) raises ValueError."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match="Limit must be greater than 0"):
        SearchQuery().query("cancer").limit(0)


def test_limit_negative_raises_valueerror() -> None:
    """Test that negative limits raise ValueError."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match="Limit must be greater than 0"):
        SearchQuery().query("cancer").limit(-1)

    with pytest.raises(ValueError, match="Limit must be greater than 0"):
        SearchQuery().query("cancer").limit(-100)


def test_limit_exceeds_10000_raises_valueerror() -> None:
    """Test that limits > 10000 raise ValueError."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match="Limit should not exceed 10,000"):
        SearchQuery().query("cancer").limit(10001)

    with pytest.raises(ValueError, match="Limit should not exceed 10,000"):
        SearchQuery().query("cancer").limit(20000)


def test_build_empty_query_raises_valueerror() -> None:
    """Test that build() on empty query raises ValueError."""
    from pubmed_client import SearchQuery

    query = SearchQuery()
    with pytest.raises(ValueError, match="Cannot build query: no search terms provided"):
        query.build()


def test_build_only_none_terms_raises_valueerror() -> None:
    """Test that query with only None/empty terms raises ValueError."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query(None).query("").query("   ")
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_build_single_term() -> None:
    """Test building query with single term."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("machine learning")
    assert query.build() == "machine learning"


def test_build_multiple_terms_space_separated() -> None:
    """Test that multiple terms are space-separated in build output."""
    from pubmed_client import SearchQuery

    # Via multiple query() calls
    query1 = SearchQuery().query("cancer").query("treatment").query("outcomes")
    assert query1.build() == "cancer treatment outcomes"

    # Via terms() batch addition
    query2 = SearchQuery().terms(["cancer", "treatment", "outcomes"])
    assert query2.build() == "cancer treatment outcomes"

    # Mixed approach
    query3 = SearchQuery().query("cancer").terms(["treatment", "outcomes"])
    assert query3.build() == "cancer treatment outcomes"


def test_method_chaining_returns_self() -> None:
    """Test that builder methods return self for fluent API."""
    from pubmed_client import SearchQuery

    query = SearchQuery()

    # Test that chaining works
    result = query.query("test").limit(10)
    assert result is query  # Should return same instance

    # Complex chaining
    final_query = (
        SearchQuery().query("covid-19").query("vaccine").terms(["efficacy", "safety"]).limit(50)
    )
    assert final_query.build() == "covid-19 vaccine efficacy safety"
