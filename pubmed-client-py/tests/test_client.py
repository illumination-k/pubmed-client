"""Tests for PubMed and PMC clients."""

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
