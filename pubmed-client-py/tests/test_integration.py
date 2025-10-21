"""Integration tests for PubMed client (requires network access).

These tests make real API calls to NCBI and are marked with @pytest.mark.integration.
Run with: pytest -m integration

Set NCBI_API_KEY environment variable to use an API key for higher rate limits.
"""

import os

import pytest

import pubmed_client


@pytest.fixture
def client() -> pubmed_client.Client:
    """Create a configured client for testing."""
    config = pubmed_client.ClientConfig()
    config.with_email("test@example.com").with_tool("pubmed-client-py-tests")

    # Use API key if available
    api_key = os.environ.get("NCBI_API_KEY")
    if api_key:
        config.with_api_key(api_key)

    # Use conservative rate limit for tests
    config.with_rate_limit(1.0)  # 1 request per second

    return pubmed_client.Client.with_config(config)


@pytest.mark.integration
class TestPubMedIntegration:
    """Integration tests for PubMed API."""

    def test_fetch_article(self, client: pubmed_client.Client) -> None:
        """Test fetching a single article by PMID."""
        # PMID 31978945 - COVID-19 related article
        article = client.pubmed.fetch_article("31978945")

        assert article is not None
        assert article.pmid == "31978945"
        assert article.title is not None
        assert len(article.title) > 0
        assert article.journal is not None
        assert article.pub_date is not None

        # Check authors
        authors = article.authors()
        assert isinstance(authors, list)
        assert len(authors) > 0

        # Check article types
        article_types = article.article_types()
        assert isinstance(article_types, list)

    def test_search_and_fetch(self, client: pubmed_client.Client) -> None:
        """Test searching for articles and fetching metadata."""
        # Search for a small number of articles
        articles = client.pubmed.search_and_fetch("machine learning", 3)

        assert isinstance(articles, list)
        assert len(articles) <= 3

        if len(articles) > 0:
            article = articles[0]
            assert article.pmid is not None
            assert article.title is not None
            assert isinstance(article.authors(), list)

    def test_get_database_list(self, client: pubmed_client.Client) -> None:
        """Test getting list of available databases."""
        databases = client.pubmed.get_database_list()

        assert isinstance(databases, list)
        assert len(databases) > 0
        assert "pubmed" in databases
        assert "pmc" in databases

    def test_get_database_info(self, client: pubmed_client.Client) -> None:
        """Test getting detailed database information."""
        info = client.pubmed.get_database_info("pubmed")

        assert info is not None
        assert info.name == "pubmed"
        assert info.description is not None
        assert len(info.description) > 0

    def test_get_related_articles(self, client: pubmed_client.Client) -> None:
        """Test getting related articles."""
        related = client.pubmed.get_related_articles([31978945])

        assert related is not None
        assert isinstance(related.source_pmids, list)
        assert isinstance(related.related_pmids, list)
        assert related.link_type is not None

        # Test __len__ method
        assert len(related) == len(related.related_pmids)

    def test_get_pmc_links(self, client: pubmed_client.Client) -> None:
        """Test getting PMC links for PMIDs."""
        links = client.pubmed.get_pmc_links([31978945])

        assert links is not None
        assert isinstance(links.source_pmids, list)
        assert isinstance(links.pmc_ids, list)

        # Test __len__ method
        assert len(links) == len(links.pmc_ids)

    def test_get_citations(self, client: pubmed_client.Client) -> None:
        """Test getting citing articles."""
        citations = client.pubmed.get_citations([31978945])

        assert citations is not None
        assert isinstance(citations.source_pmids, list)
        assert isinstance(citations.citing_pmids, list)

        # Test __len__ method
        assert len(citations) == len(citations.citing_pmids)


@pytest.mark.integration
class TestPmcIntegration:
    """Integration tests for PMC API."""

    def test_check_pmc_availability(self, client: pubmed_client.Client) -> None:
        """Test checking PMC availability."""
        # PMID 31978945 has PMC full text
        pmcid = client.pmc.check_pmc_availability("31978945")

        # May or may not have PMC full text
        if pmcid is not None:
            assert pmcid.startswith("PMC")

    def test_fetch_full_text(self, client: pubmed_client.Client) -> None:
        """Test fetching PMC full text."""
        # PMC7906746 is known to exist
        full_text = client.pmc.fetch_full_text("PMC7906746")

        assert full_text is not None
        assert full_text.pmcid == "PMC7906746"
        assert full_text.title is not None
        assert len(full_text.title) > 0

        # Check authors
        authors = full_text.authors()
        assert isinstance(authors, list)

        # Check sections
        sections = full_text.sections()
        assert isinstance(sections, list)

        # Check figures
        figures = full_text.figures()
        assert isinstance(figures, list)

        # Check tables
        tables = full_text.tables()
        assert isinstance(tables, list)

        # Check references
        references = full_text.references()
        assert isinstance(references, list)


@pytest.mark.integration
@pytest.mark.slow
class TestCombinedIntegration:
    """Integration tests for combined operations."""

    def test_search_with_full_text(self, client: pubmed_client.Client) -> None:
        """Test searching and fetching full text manually."""
        # Search for a small number of articles
        articles = client.pubmed.search_and_fetch("CRISPR", 2)

        assert isinstance(articles, list)
        assert len(articles) <= 2

        for article in articles:
            assert article is not None
            assert article.pmid is not None

            # Try to get full text for articles that have PMC versions
            pmcid = client.pmc.check_pmc_availability(article.pmid)
            if pmcid is not None:
                full_text = client.pmc.fetch_full_text(pmcid)
                assert full_text is not None
                assert full_text.pmcid is not None
                assert full_text.title is not None
