"""Pytest configuration and shared fixtures."""

import pytest


def pytest_configure(config):
    """Configure pytest with custom markers."""
    config.addinivalue_line("markers", "integration: mark test as integration test (requires network)")
    config.addinivalue_line("markers", "slow: mark test as slow running")


@pytest.fixture(scope="session")
def pubmed_client():
    """Create a basic PubMed client for testing."""
    import pubmed_client

    return pubmed_client.PubMedClient()


@pytest.fixture(scope="session")
def pmc_client():
    """Create a basic PMC client for testing."""
    import pubmed_client

    return pubmed_client.PmcClient()


@pytest.fixture(scope="session")
def combined_client():
    """Create a basic combined client for testing."""
    import pubmed_client

    return pubmed_client.Client()
