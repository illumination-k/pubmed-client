"""Unit tests for SearchQuery Python bindings."""

import pytest

from pubmed_client import SearchQuery


def test_searchquery_constructor_creates_empty_query() -> None:
    """Test that SearchQuery() creates an empty query that raises ValueError on build()."""
    query = SearchQuery()
    # Empty query should raise ValueError when build() is called
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_query_single_term() -> None:
    """Test adding a single search term."""
    query = SearchQuery().query("covid-19")
    assert query.build() == "covid-19"


def test_query_multiple_calls_accumulate() -> None:
    """Test that multiple query() calls accumulate terms."""
    query = SearchQuery().query("covid-19").query("treatment")
    assert query.build() == "covid-19 treatment"


def test_terms_batch_addition() -> None:
    """Test adding multiple terms at once via terms() method."""
    query = SearchQuery().terms(["covid-19", "vaccine", "efficacy"])
    assert query.build() == "covid-19 vaccine efficacy"


def test_query_none_filtered_silently() -> None:
    """Test that None values in query() are silently filtered."""
    query = SearchQuery().query(None).query("covid-19")
    assert query.build() == "covid-19"


def test_terms_none_filtered_silently() -> None:
    """Test that None values in terms() list are silently filtered."""

    terms: list[str | None] = [None, "covid-19", None, "vaccine"]
    query = SearchQuery().terms(terms)
    assert query.build() == "covid-19 vaccine"


def test_query_empty_string_filtered() -> None:
    """Test that empty strings and whitespace-only strings are filtered."""
    query = SearchQuery().query("").query("   ").query("cancer")
    assert query.build() == "cancer"


def test_limit_valid_values() -> None:
    """Test that valid limit values are accepted."""
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
    query = SearchQuery().query("cancer").limit(None)
    # Should not raise error, None means "use default"
    assert query.build() == "cancer"


def test_limit_zero_raises_valueerror() -> None:
    """Test that limit(0) raises ValueError."""
    with pytest.raises(ValueError, match="Limit must be greater than 0"):
        SearchQuery().query("cancer").limit(0)


def test_limit_negative_raises_valueerror() -> None:
    """Test that negative limits are rejected by PyO3 type system."""
    # PyO3 rejects negative values for usize parameters before our validation runs
    with pytest.raises(OverflowError, match="can't convert negative int to unsigned"):
        SearchQuery().query("cancer").limit(-1)


def test_limit_exceeds_10000_raises_valueerror() -> None:
    """Test that limits > 10000 raise ValueError."""
    with pytest.raises(ValueError, match="Limit should not exceed 10,000"):
        SearchQuery().query("cancer").limit(10001)

    with pytest.raises(ValueError, match="Limit should not exceed 10,000"):
        SearchQuery().query("cancer").limit(20000)


def test_build_empty_query_raises_valueerror() -> None:
    """Test that build() on empty query raises ValueError."""
    query = SearchQuery()
    with pytest.raises(ValueError, match="Cannot build query: no search terms provided"):
        query.build()


def test_build_only_none_terms_raises_valueerror() -> None:
    """Test that query with only None/empty terms raises ValueError."""
    query = SearchQuery().query(None).query("").query("   ")
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_build_single_term() -> None:
    """Test building query with single term."""
    query = SearchQuery().query("machine learning")
    assert query.build() == "machine learning"


def test_build_multiple_terms_space_separated() -> None:
    """Test that multiple terms are space-separated in build output."""
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
    query = SearchQuery().query("covid-19").published_in_year(2024)
    result = query.build()
    assert "2024[pdat]" in result
    assert "covid-19" in result


def test_published_between_with_both_years() -> None:
    """Test that published_between() with both years generates correct date range filter."""
    query = SearchQuery().query("cancer").published_between(2020, 2023)
    result = query.build()
    assert "2020:2023[pdat]" in result
    assert "cancer" in result


def test_published_between_with_none_end_year() -> None:
    """Test that published_between() with None end_year uses 3000 as upper bound."""
    query = SearchQuery().query("diabetes").published_between(2020, None)
    result = query.build()
    assert "2020:3000[pdat]" in result
    assert "diabetes" in result


def test_published_after() -> None:
    """Test that published_after() generates correct open-ended date range."""
    query = SearchQuery().query("treatment").published_after(2020)
    result = query.build()
    assert "2020:3000[pdat]" in result
    assert "treatment" in result


def test_published_before() -> None:
    """Test that published_before() generates correct upper-bounded date range."""
    query = SearchQuery().query("epidemiology").published_before(2020)
    result = query.build()
    assert "1900:2020[pdat]" in result
    assert "epidemiology" in result


@pytest.mark.parametrize("invalid_year", [999, 1799, 3001, 5000])
def test_invalid_years_raise_valueerror(invalid_year: int) -> None:
    """Test that years outside 1800-3000 range raise ValueError."""
    with pytest.raises(ValueError, match="Year must be between 1800 and 3000"):
        SearchQuery().query("topic").published_in_year(invalid_year)


def test_invalid_date_range_raises_valueerror() -> None:
    """Test that start_year > end_year raises ValueError."""
    with pytest.raises(ValueError, match=r"Start year.*must be.*end year"):
        SearchQuery().query("topic").published_between(2024, 2020)


# ================================================================================================
# Article Type Filtering Tests (User Story 2)
# ================================================================================================


def test_article_type_single() -> None:
    """Test that article_type() with valid type generates correct filter."""
    query = SearchQuery().query("cancer").article_type("Clinical Trial")
    result = query.build()
    assert "Clinical Trial[pt]" in result
    assert "cancer" in result


def test_article_type_case_insensitive() -> None:
    """Test that article_type() handles case-insensitive input."""
    query = SearchQuery().query("diabetes").article_type("clinical trial")
    result = query.build()
    assert "Clinical Trial[pt]" in result


def test_article_types_multiple() -> None:
    """Test that article_types() with multiple types generates OR combination."""
    query = SearchQuery().query("treatment").article_types(["RCT", "Meta-Analysis"])
    result = query.build()
    # Should contain OR combination
    assert "Randomized Controlled Trial[pt]" in result
    assert "Meta-Analysis[pt]" in result
    assert " OR " in result


def test_article_types_empty_list() -> None:
    """Test that article_types() with empty list is ignored."""
    query = SearchQuery().query("research").article_types([])
    result = query.build()
    # Should just have the search term, no article type filter
    assert result == "research"


@pytest.mark.parametrize(
    ("article_type_name", "expected_tag"),
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
    query = SearchQuery().query("topic").article_type(article_type_name)
    result = query.build()
    assert expected_tag in result


def test_invalid_article_type_raises_valueerror() -> None:
    """Test that invalid article type raises ValueError with helpful message."""
    with pytest.raises(ValueError, match=r"Invalid article type.*Supported types"):
        SearchQuery().query("topic").article_type("Invalid Type")


# ================================================================================================
# Open Access Filtering Tests (User Story 3)
# ================================================================================================


def test_free_full_text_only() -> None:
    """Test that free_full_text_only() adds free full text filter."""
    query = SearchQuery().query("cancer").free_full_text_only()
    result = query.build()
    assert "cancer" in result
    assert "free full text[sb]" in result


def test_full_text_only() -> None:
    """Test that full_text_only() adds full text filter."""
    query = SearchQuery().query("diabetes").full_text_only()
    result = query.build()
    assert "diabetes" in result
    assert "full text[sb]" in result


def test_pmc_only() -> None:
    """Test that pmc_only() adds PMC filter."""
    query = SearchQuery().query("treatment").pmc_only()
    result = query.build()
    assert "treatment" in result
    assert "pmc[sb]" in result


def test_multiple_access_filters_can_be_combined() -> None:
    """Test that multiple access filters can be combined (though unusual)."""
    query = SearchQuery().query("research").free_full_text_only().pmc_only()
    result = query.build()
    assert "research" in result
    assert "free full text[sb]" in result
    assert "pmc[sb]" in result


def test_access_filters_with_other_filters() -> None:
    """Test that access filters work with date and article type filters."""

    query = (
        SearchQuery()
        .query("covid-19")
        .published_in_year(2024)
        .article_type("Review")
        .free_full_text_only()
    )
    result = query.build()
    assert "covid-19" in result
    assert "2024[pdat]" in result
    assert "Review[pt]" in result
    assert "free full text[sb]" in result


# ================================================================================================
# Boolean Logic Tests (User Story 4 - AND, OR, NOT operations)
# ================================================================================================


def test_and_operation() -> None:
    """Test that and_() combines two queries with AND logic."""

    q1 = SearchQuery().query("covid-19")
    q2 = SearchQuery().query("vaccine")
    combined = q1.and_(q2)
    assert combined.build() == "(covid-19) AND (vaccine)"


def test_or_operation() -> None:
    """Test that or_() combines two queries with OR logic."""

    q1 = SearchQuery().query("diabetes")
    q2 = SearchQuery().query("hypertension")
    combined = q1.or_(q2)
    assert combined.build() == "(diabetes) OR (hypertension)"


def test_negate_operation() -> None:
    """Test that negate() wraps query with NOT logic."""
    query = SearchQuery().query("cancer").negate()
    assert query.build() == "NOT (cancer)"


def test_exclude_operation() -> None:
    """Test that exclude() filters out unwanted results."""

    base = SearchQuery().query("cancer treatment")
    exclude = SearchQuery().query("animal studies")
    filtered = base.exclude(exclude)
    assert filtered.build() == "(cancer treatment) NOT (animal studies)"


def test_group_operation() -> None:
    """Test that group() wraps query in parentheses."""
    query = SearchQuery().query("cancer").group()
    assert query.build() == "(cancer)"


def test_complex_boolean_chain() -> None:
    """Test complex chaining of boolean operations."""

    ai_query = SearchQuery().query("machine learning")
    medicine_query = SearchQuery().query("medicine")
    exclude_query = SearchQuery().query("veterinary")

    final_query = ai_query.and_(medicine_query).exclude(exclude_query).group()

    assert final_query.build() == "(((machine learning) AND (medicine)) NOT (veterinary))"


def test_and_with_empty_first_query() -> None:
    """Test that and_() with empty first query returns the second query."""

    q1 = SearchQuery()
    q2 = SearchQuery().query("test")
    combined = q1.and_(q2)
    assert combined.build() == "test"


def test_or_with_empty_second_query() -> None:
    """Test that or_() with empty second query returns the first query."""

    q1 = SearchQuery().query("test")
    q2 = SearchQuery()
    combined = q1.or_(q2)
    assert combined.build() == "test"


def test_negate_empty_query() -> None:
    """Test that negating an empty query produces empty result."""
    query = SearchQuery().negate()
    # Empty query still raises ValueError
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_exclude_with_empty_base() -> None:
    """Test that excluding from empty base returns empty."""

    base = SearchQuery()
    exclude = SearchQuery().query("test")
    filtered = base.exclude(exclude)
    # Empty query still raises ValueError
    with pytest.raises(ValueError, match="Cannot build query"):
        filtered.build()


def test_exclude_with_empty_excluded() -> None:
    """Test that excluding empty query returns base unchanged."""

    base = SearchQuery().query("test")
    exclude = SearchQuery()
    filtered = base.exclude(exclude)
    assert filtered.build() == "test"


def test_group_empty_query() -> None:
    """Test that grouping empty query produces empty result."""
    query = SearchQuery().group()
    # Empty query still raises ValueError
    with pytest.raises(ValueError, match="Cannot build query"):
        query.build()


def test_deep_boolean_nesting() -> None:
    """Test deeply nested boolean operations."""

    q1 = SearchQuery().query("a")
    q2 = SearchQuery().query("b")
    q3 = SearchQuery().query("c")
    q4 = SearchQuery().query("d")

    nested = q1.and_(q2).or_(q3.and_(q4))
    assert nested.build() == "((a) AND (b)) OR ((c) AND (d))"


def test_boolean_operations_preserve_limit() -> None:
    """Test that boolean operations preserve the higher limit."""

    q1 = SearchQuery().query("covid").limit(10)
    q2 = SearchQuery().query("vaccine").limit(50)
    combined = q1.and_(q2)

    # Should use higher limit (50)
    assert combined.get_limit() == 50


def test_and_multiple_operations() -> None:
    """Test chaining multiple AND operations."""

    result = (
        SearchQuery()
        .query("cancer")
        .and_(SearchQuery().query("treatment"))
        .and_(SearchQuery().query("outcomes"))
    )
    assert result.build() == "((cancer) AND (treatment)) AND (outcomes)"


def test_or_multiple_operations() -> None:
    """Test chaining multiple OR operations."""

    result = (
        SearchQuery()
        .query("cancer")
        .or_(SearchQuery().query("tumor"))
        .or_(SearchQuery().query("oncology"))
    )
    assert result.build() == "((cancer) OR (tumor)) OR (oncology)"


def test_mixed_boolean_operations() -> None:
    """Test mixing AND and OR operations."""
    # (covid OR sars) AND vaccine
    covid_or_sars = SearchQuery().query("covid").or_(SearchQuery().query("sars"))
    result = covid_or_sars.and_(SearchQuery().query("vaccine"))

    assert result.build() == "((covid) OR (sars)) AND (vaccine)"


def test_exclude_multiple_terms() -> None:
    """Test excluding multiple terms sequentially."""

    result = (
        SearchQuery()
        .query("therapy")
        .exclude(SearchQuery().query("animal studies"))
        .exclude(SearchQuery().query("in vitro"))
    )
    # Each exclude wraps the previous result
    assert result.build() == "((therapy) NOT (animal studies)) NOT (in vitro)"


def test_boolean_with_filters() -> None:
    """Test boolean operations combined with date and type filters."""

    q1 = SearchQuery().query("covid-19").published_in_year(2024)
    q2 = SearchQuery().query("vaccine").article_type("Clinical Trial")

    result = q1.and_(q2)
    built = result.build()

    # Should contain both queries and their filters
    assert "covid-19" in built
    assert "vaccine" in built
    assert "2024[pdat]" in built
    assert "Clinical Trial[pt]" in built
    assert " AND " in built


def test_complex_real_world_query() -> None:
    """Test a complex real-world research query."""
    # Find recent clinical trials for COVID-19 treatments, excluding animal studies
    covid_query = SearchQuery().query("covid-19").query("treatment").published_after(2020)
    clinical_trials = SearchQuery().article_type("Clinical Trial")
    animal_studies = SearchQuery().query("animal studies")

    result = covid_query.and_(clinical_trials).exclude(animal_studies)

    built = result.build()
    assert "covid-19" in built
    assert "treatment" in built
    assert "2020:3000[pdat]" in built
    assert "Clinical Trial[pt]" in built
    assert "animal studies" in built
    assert " NOT " in built


def test_precedence_control_with_group() -> None:
    """Test controlling operator precedence with group()."""

    q1 = SearchQuery().query("a").or_(SearchQuery().query("b")).group()
    q2 = SearchQuery().query("c").or_(SearchQuery().query("d")).group()
    result = q1.and_(q2)

    assert result.build() == "(((a) OR (b))) AND (((c) OR (d)))"
