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
    """Test that negative limits are rejected by PyO3 type system."""
    from pubmed_client import SearchQuery

    # PyO3 rejects negative values for usize parameters before our validation runs
    with pytest.raises(OverflowError, match="can't convert negative int to unsigned"):
        SearchQuery().query("cancer").limit(-1)


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


# ================================================================================================
# Date Filtering Tests (User Story 1)
# ================================================================================================


def test_published_in_year() -> None:
    """Test that published_in_year() generates correct date filter."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("covid-19").published_in_year(2024)
    result = query.build()
    assert "2024[pdat]" in result
    assert "covid-19" in result


def test_published_between_with_both_years() -> None:
    """Test that published_between() with both years generates correct date range filter."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("cancer").published_between(2020, 2023)
    result = query.build()
    assert "2020:2023[pdat]" in result
    assert "cancer" in result


def test_published_between_with_none_end_year() -> None:
    """Test that published_between() with None end_year uses 3000 as upper bound."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("diabetes").published_between(2020, None)
    result = query.build()
    assert "2020:3000[pdat]" in result
    assert "diabetes" in result


def test_published_after() -> None:
    """Test that published_after() generates correct open-ended date range."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("treatment").published_after(2020)
    result = query.build()
    assert "2020:3000[pdat]" in result
    assert "treatment" in result


def test_published_before() -> None:
    """Test that published_before() generates correct upper-bounded date range."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("epidemiology").published_before(2020)
    result = query.build()
    assert "1900:2020[pdat]" in result
    assert "epidemiology" in result


@pytest.mark.parametrize("invalid_year", [999, 1799, 3001, 5000])
def test_invalid_years_raise_valueerror(invalid_year: int) -> None:
    """Test that years outside 1800-3000 range raise ValueError."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match="Year must be between 1800 and 3000"):
        SearchQuery().query("topic").published_in_year(invalid_year)


def test_invalid_date_range_raises_valueerror() -> None:
    """Test that start_year > end_year raises ValueError."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match=r"Start year.*must be.*end year"):
        SearchQuery().query("topic").published_between(2024, 2020)


# ================================================================================================
# Article Type Filtering Tests (User Story 2)
# ================================================================================================


def test_article_type_single() -> None:
    """Test that article_type() with valid type generates correct filter."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("cancer").article_type("Clinical Trial")
    result = query.build()
    assert "Clinical Trial[pt]" in result
    assert "cancer" in result


def test_article_type_case_insensitive() -> None:
    """Test that article_type() handles case-insensitive input."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("diabetes").article_type("clinical trial")
    result = query.build()
    assert "Clinical Trial[pt]" in result


def test_article_types_multiple() -> None:
    """Test that article_types() with multiple types generates OR combination."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("treatment").article_types(["RCT", "Meta-Analysis"])
    result = query.build()
    # Should contain OR combination
    assert "Randomized Controlled Trial[pt]" in result
    assert "Meta-Analysis[pt]" in result
    assert " OR " in result


def test_article_types_empty_list() -> None:
    """Test that article_types() with empty list is ignored."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("research").article_types([])
    result = query.build()
    # Should just have the search term, no article type filter
    assert result == "research"


@pytest.mark.parametrize(
    "article_type_name,expected_tag",
    [
        ("Clinical Trial", "Clinical Trial[pt]"),
        ("Review", "Review[pt]"),
        ("Systematic Review", "Systematic Review[pt]"),
        ("Meta-Analysis", "Meta-Analysis[pt]"),
        ("Case Reports", "Case Reports[pt]"),
        ("Randomized Controlled Trial", "Randomized Controlled Trial[pt]"),
        ("Observational Study", "Observational Study[pt]"),
    ],
)
def test_all_article_types_supported(article_type_name: str, expected_tag: str) -> None:
    """Test that all 7 supported article types work correctly."""
    from pubmed_client import SearchQuery

    query = SearchQuery().query("topic").article_type(article_type_name)
    result = query.build()
    assert expected_tag in result


def test_invalid_article_type_raises_valueerror() -> None:
    """Test that invalid article type raises ValueError with helpful message."""
    from pubmed_client import SearchQuery

    with pytest.raises(ValueError, match=r"Invalid article type.*Supported types"):
        SearchQuery().query("topic").article_type("Invalid Type")
