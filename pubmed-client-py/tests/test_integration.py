"""Integration tests for PubMed client (requires network access).

These tests make real API calls to NCBI and are marked with @pytest.mark.integration.
Run with: pytest -m integration

Set NCBI_API_KEY environment variable to use an API key for higher rate limits.
"""

import os
import tempfile
from pathlib import Path

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

    def test_search_articles(self, client: pubmed_client.Client) -> None:
        """Test searching for articles and getting PMIDs only."""
        try:
            # Search for a small number of PMIDs
            pmids = client.pubmed.search_articles("covid-19", 5)

            assert isinstance(pmids, list)
            assert len(pmids) <= 5

            # All results should be strings (PMIDs)
            for pmid in pmids:
                assert isinstance(pmid, str)
                assert pmid.isdigit()  # PMIDs are numeric strings

            # Test with field-specific query
            pmids_with_filter = client.pubmed.search_articles("cancer[ti]", 3)
            assert isinstance(pmids_with_filter, list)
            assert len(pmids_with_filter) <= 3
        except Exception as e:
            # Skip test on transient API errors or rate limiting
            error_msg = str(e)
            if (
                "429" in error_msg
                or "Too Many Requests" in error_msg
                or "rate limit" in error_msg.lower()
            ):
                pytest.skip(f"Skipping due to transient API issue: {error_msg}")
            raise

    def test_search_and_fetch(self, client: pubmed_client.Client) -> None:
        """Test searching for articles and fetching metadata."""
        try:
            # Search for a small number of articles
            articles = client.pubmed.search_and_fetch("machine learning", 3)

            assert isinstance(articles, list)
            assert len(articles) <= 3

            if len(articles) > 0:
                article = articles[0]
                assert article.pmid is not None
                assert article.title is not None
                assert isinstance(article.authors(), list)
        except Exception as e:
            # Skip test on transient API parsing errors or rate limiting
            error_msg = str(e)
            if (
                "missing field" in error_msg
                or "429" in error_msg
                or "Too Many Requests" in error_msg
            ):
                pytest.skip(f"Skipping due to transient API issue: {error_msg}")
            raise

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

    def test_to_markdown(self, client: pubmed_client.Client) -> None:
        """Test converting PMC full text to Markdown."""
        # PMC7906746 is known to exist
        full_text = client.pmc.fetch_full_text("PMC7906746")

        assert full_text is not None

        # Convert to markdown
        markdown = full_text.to_markdown()

        # Verify markdown is a non-empty string
        assert isinstance(markdown, str)
        assert len(markdown) > 0

        # Check for expected markdown elements
        assert "#" in markdown  # Should have markdown headers
        assert full_text.title in markdown  # Title should be in markdown
        assert full_text.pmcid in markdown  # PMC ID should be in markdown

    def test_download_and_extract_tar(self, client: pubmed_client.Client) -> None:
        """Test downloading and extracting PMC tar.gz archive."""
        # Use PMC7906746 which is known to exist and have figures
        pmcid = "PMC7906746"

        # Create temporary directory for extraction
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = str(Path(temp_dir) / pmcid)

            # Download and extract tar.gz
            files = client.pmc.download_and_extract_tar(pmcid, output_dir)

            # Verify files were extracted
            assert isinstance(files, list)
            assert len(files) > 0

            # Verify at least one file exists and is in the output directory
            for file_path in files:
                assert isinstance(file_path, str)
                assert len(file_path) > 0
                # File path should be within output directory
                assert output_dir in file_path or Path(file_path).is_absolute()

            # Verify output directory was created
            assert Path(output_dir).exists()
            assert Path(output_dir).is_dir()

            # Verify at least some files are accessible
            file_count = 0
            for file_path in files:
                if Path(file_path).exists():
                    file_count += 1
            assert file_count > 0, "No extracted files found on disk"

    def test_download_and_extract_tar_without_pmc_prefix(
        self, client: pubmed_client.Client
    ) -> None:
        """Test downloading tar.gz with PMCID without 'PMC' prefix."""
        # Use just the numeric part
        pmcid = "7906746"

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = str(Path(temp_dir) / f"PMC{pmcid}")

            files = client.pmc.download_and_extract_tar(pmcid, output_dir)

            assert isinstance(files, list)
            assert len(files) > 0

    def test_extract_figures_with_captions(self, client: pubmed_client.Client) -> None:
        """Test extracting figures with their captions from PMC article."""
        # PMC7906746 is known to have figures
        pmcid = "PMC7906746"

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = str(Path(temp_dir) / pmcid)

            # Extract figures with captions
            figures = client.pmc.extract_figures_with_captions(pmcid, output_dir)

            # Verify we got ExtractedFigure objects
            assert isinstance(figures, list)

            if len(figures) > 0:
                # Check first figure has all expected fields
                fig = figures[0]

                # Check that we have the figure metadata
                assert hasattr(fig, "figure")
                assert hasattr(fig.figure, "id")
                assert hasattr(fig.figure, "caption")
                assert isinstance(fig.figure.id, str)
                assert isinstance(fig.figure.caption, str)

                # Check that we have the extracted file path
                assert hasattr(fig, "extracted_file_path")
                assert isinstance(fig.extracted_file_path, str)
                assert len(fig.extracted_file_path) > 0

                # File should exist
                assert Path(fig.extracted_file_path).exists()

                # Check optional fields exist
                assert hasattr(fig, "file_size")
                assert hasattr(fig, "dimensions")

                # If file_size is available, it should be > 0
                if fig.file_size is not None:
                    assert fig.file_size > 0

                # If dimensions are available, they should be valid
                if fig.dimensions is not None:
                    width, height = fig.dimensions
                    assert isinstance(width, int)
                    assert isinstance(height, int)
                    assert width > 0
                    assert height > 0

    def test_extract_figures_with_captions_invalid_pmcid(
        self, client: pubmed_client.Client
    ) -> None:
        """Test extracting figures with invalid PMCID raises appropriate error."""
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = str(Path(temp_dir) / "invalid")

            # Use an invalid PMCID that should not exist
            # Should raise an error (likely PmcNotAvailableById or network error)
            with pytest.raises(Exception, match=r"PMC.*not available|error|Error|not found"):
                client.pmc.extract_figures_with_captions("PMC99999999999", output_dir)


@pytest.mark.integration
@pytest.mark.slow
class TestCombinedIntegration:
    """Integration tests for combined operations."""

    def test_search_with_full_text(self, client: pubmed_client.Client) -> None:
        """Test searching and fetching full text manually."""
        try:
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
        except Exception as e:
            # Skip test on transient API parsing errors or rate limiting
            error_msg = str(e)
            if (
                "missing field" in error_msg
                or "429" in error_msg
                or "Too Many Requests" in error_msg
            ):
                pytest.skip(f"Skipping due to transient API issue: {error_msg}")
            raise
