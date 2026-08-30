"""Tests for the Europe PMC client bindings."""

import pytest

import pubmed_client


class TestEuropePmcClient:
    """Tests for EuropePmcClient construction and wiring."""

    def test_client_creation(self) -> None:
        """Test creating a Europe PMC client."""
        client = pubmed_client.EuropePmcClient()
        assert client is not None
        assert repr(client) == "EuropePmcClient()"

    def test_client_with_config(self) -> None:
        """Test creating a Europe PMC client with configuration."""
        config = pubmed_client.ClientConfig()
        config.with_email("test@example.com")
        client = pubmed_client.EuropePmcClient.with_config(config)
        assert client is not None

    def test_combined_client_property(self) -> None:
        """Test accessing the Europe PMC client from the combined client."""
        client = pubmed_client.Client()
        europe_pmc = client.europe_pmc
        assert europe_pmc is not None
        assert repr(europe_pmc) == "EuropePmcClient()"


class TestEuropePmcIdValidation:
    """Tests for `(source, id)` resolution.

    These run offline: an id is rejected before any request is issued, so a
    bad address surfaces as a ValueError rather than an HTTP error.
    """

    def test_empty_id_is_rejected(self) -> None:
        client = pubmed_client.EuropePmcClient()
        with pytest.raises(ValueError, match="id must not be empty"):
            client.get_references("   ")

    def test_malformed_qualified_id_is_rejected(self) -> None:
        client = pubmed_client.EuropePmcClient()
        with pytest.raises(ValueError, match="invalid Europe PMC id"):
            client.get_citations("MED/")

    def test_non_numeric_pmc_id_is_rejected(self) -> None:
        client = pubmed_client.EuropePmcClient()
        with pytest.raises(ValueError, match="invalid PMC id"):
            client.fetch_full_text("not-a-pmcid", source="PMC")

    def test_invalid_result_type_is_rejected(self) -> None:
        client = pubmed_client.EuropePmcClient()
        with pytest.raises(ValueError, match="invalid result_type"):
            client.search_page("cancer", result_type="verbose")


class TestEuropePmcSearch:
    """Live Europe PMC search tests."""

    @pytest.mark.integration
    def test_search_returns_results(self) -> None:
        client = pubmed_client.EuropePmcClient()
        results = client.search("malaria vaccine", 5)

        assert 0 < len(results) <= 5
        first = results[0]
        assert first.id
        assert first.source
        assert first.europe_pmc_id == f"{first.source}/{first.id}"
        assert isinstance(first.extra(), dict)

    @pytest.mark.integration
    def test_search_page_exposes_cursor(self) -> None:
        client = pubmed_client.EuropePmcClient()
        page = client.search_page("malaria vaccine", page_size=5)

        assert page.hit_count > 0
        assert len(page) == len(page.results())
        # A first page of a large result set always has a follow-on cursor.
        assert page.next_cursor_mark

    @pytest.mark.integration
    def test_core_result_type_carries_extra_fields(self) -> None:
        client = pubmed_client.EuropePmcClient()
        results = client.search_all("malaria vaccine", 3, result_type="core")

        assert results
        # `core` returns far more than is modelled; whatever it adds lands in
        # `extra()` rather than being dropped.
        assert any(record.extra() for record in results)


class TestEuropePmcRecords:
    """Live Europe PMC record tests."""

    @pytest.mark.integration
    def test_fetch_full_text(self) -> None:
        client = pubmed_client.EuropePmcClient()
        article = client.fetch_full_text("PMC3258128")

        assert article.pmcid == "PMC3258128"
        assert article.title

    @pytest.mark.integration
    def test_fetch_full_text_xml(self) -> None:
        client = pubmed_client.EuropePmcClient()
        xml = client.fetch_full_text_xml("PMC3258128")

        assert "<article" in xml

    @pytest.mark.integration
    def test_get_references(self) -> None:
        client = pubmed_client.EuropePmcClient()
        references = client.get_references("PMC3258128")

        assert references
        assert any(reference.title for reference in references)

    @pytest.mark.integration
    def test_get_references_page_reports_hit_count(self) -> None:
        client = pubmed_client.EuropePmcClient()
        hit_count, references = client.get_references_page("PMC3258128", page=1, page_size=5)

        assert hit_count > 0
        assert len(references) <= 5

    @pytest.mark.integration
    def test_get_citations(self) -> None:
        client = pubmed_client.EuropePmcClient()
        citations = client.get_citations("PMC3258128")

        assert citations
        assert any(citation.title for citation in citations)

    @pytest.mark.integration
    def test_get_database_links(self) -> None:
        client = pubmed_client.EuropePmcClient()
        links = client.get_database_links("PMC3258128")

        # A record may legitimately have no cross-references; assert on shape
        # rather than presence.
        for link in links:
            assert link.db_name
            assert isinstance(link.info(), list)
