"""Integration tests for SearchQuery Python bindings.

These tests verify end-to-end query construction scenarios that match
real-world usage patterns.
"""


def test_build_query_for_pubmed_search() -> None:
    """Test building a query suitable for PubMed search."""
    from pubmed_client import SearchQuery

    # Simulate building a query for a literature review
    query = (
        SearchQuery().query("machine learning").query("healthcare").query("diagnosis").limit(100)
    )

    query_string = query.build()
    assert query_string == "machine learning healthcare diagnosis"

    # Verify the query string format matches PubMed expectations
    # (space-separated terms = OR logic in PubMed)
    assert " " in query_string
    assert "machine learning" in query_string
    assert "healthcare" in query_string
    assert "diagnosis" in query_string


def test_complex_query_construction() -> None:
    """Test constructing a complex query with multiple term additions."""
    from pubmed_client import SearchQuery

    # Simulate a researcher building a comprehensive search query
    base_terms = ["covid-19", "sars-cov-2"]
    treatment_terms = ["treatment", "therapy", "intervention"]

    query = (
        SearchQuery().terms(base_terms).terms(treatment_terms).query("clinical trial").limit(500)
    )

    result = query.build()

    # Verify all terms are present
    for term in base_terms + treatment_terms + ["clinical trial"]:
        assert term in result

    # Verify space-separated format
    expected = "covid-19 sars-cov-2 treatment therapy intervention clinical trial"
    assert result == expected


def test_conditional_query_building() -> None:
    """Test building queries with conditional term addition."""
    from pubmed_client import SearchQuery

    # Simulate dynamic query building based on user preferences
    def build_research_query(
        base_term: str,
        include_treatment: bool = False,
        include_prevention: bool = False,
        max_results: int = 20,
    ) -> str:
        query = SearchQuery().query(base_term)

        if include_treatment:
            query = query.query("treatment")

        if include_prevention:
            query = query.query("prevention")

        return query.limit(max_results).build()

    # Test with no optional terms
    query1 = build_research_query("diabetes")
    assert query1 == "diabetes"

    # Test with treatment only
    query2 = build_research_query("diabetes", include_treatment=True)
    assert query2 == "diabetes treatment"

    # Test with both optional terms
    query3 = build_research_query("diabetes", include_treatment=True, include_prevention=True)
    assert query3 == "diabetes treatment prevention"

    # Test with None filtering
    terms = ["cancer"]
    optional_term = None  # Might come from user input

    query4 = SearchQuery().terms(terms).query(optional_term)
    assert query4.build() == "cancer"  # None filtered out


def test_date_filtering_integration() -> None:
    """Test integration of date filtering with search terms."""
    from pubmed_client import SearchQuery

    # Test recent research query with date filter
    query = SearchQuery().query("covid-19").query("vaccine").published_between(2020, 2024).limit(50)

    result = query.build()

    # Verify search terms are present
    assert "covid-19" in result
    assert "vaccine" in result

    # Verify date filter is present
    assert "2020:2024[pdat]" in result

    # Test combining multiple date filter types
    query2 = SearchQuery().query("cancer treatment").published_after(2015)

    result2 = query2.build()
    assert "cancer treatment" in result2
    assert "2015:3000[pdat]" in result2
