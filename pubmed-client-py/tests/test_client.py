"""Tests for PubMed and PMC clients."""

import pytest

import pubmed_client


class TestPubMedClient:
    """Tests for PubMedClient."""

    def test_client_creation(self) -> None:
        """Test creating a PubMed client."""
        client = pubmed_client.PubMedClient()
        assert client is not None
        assert repr(client) == "PubMedClient()"

    def test_client_with_config(self) -> None:
        """Test creating a PubMed client with configuration."""
        config = pubmed_client.ClientConfig()
        config.with_email("test@example.com")
        client = pubmed_client.PubMedClient.with_config(config)
        assert client is not None


class TestPmcClient:
    """Tests for PmcClient."""

    def test_client_creation(self) -> None:
        """Test creating a PMC client."""
        client = pubmed_client.PmcClient()
        assert client is not None
        assert repr(client) == "PmcClient()"

    def test_client_with_config(self) -> None:
        """Test creating a PMC client with configuration."""
        config = pubmed_client.ClientConfig()
        config.with_email("test@example.com")
        client = pubmed_client.PmcClient.with_config(config)
        assert client is not None


class TestCombinedClient:
    """Tests for combined Client."""

    def test_client_creation(self) -> None:
        """Test creating a combined client."""
        client = pubmed_client.Client()
        assert client is not None
        assert repr(client) == "Client()"

    def test_client_with_config(self) -> None:
        """Test creating a combined client with configuration."""
        config = pubmed_client.ClientConfig()
        config.with_email("test@example.com")
        client = pubmed_client.Client.with_config(config)
        assert client is not None

    def test_client_pubmed_property(self) -> None:
        """Test accessing PubMed client from combined client."""
        client = pubmed_client.Client()
        pubmed = client.pubmed
        assert pubmed is not None
        assert repr(pubmed) == "PubMedClient()"

    def test_client_pmc_property(self) -> None:
        """Test accessing PMC client from combined client."""
        client = pubmed_client.Client()
        pmc = client.pmc
        assert pmc is not None
        assert repr(pmc) == "PmcClient()"


class TestCitationExport:
    """Tests for citation export on PubMedArticle."""

    @pytest.mark.integration
    def test_export_formats(self) -> None:
        """Fetch a known article and export it in all four citation formats."""
        client = pubmed_client.PubMedClient()
        article = client.fetch_article("31978945")

        bibtex = article.to_bibtex()
        assert bibtex.startswith("@article{")

        ris = article.to_ris()
        assert "TY  -" in ris

        nbib = article.to_nbib()
        assert "PMID-" in nbib

        csl_json = article.to_csl_json()
        assert csl_json.strip().startswith("{")
