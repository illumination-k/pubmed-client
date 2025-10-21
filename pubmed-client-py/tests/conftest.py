"""Pytest configuration and shared fixtures."""

import pytest

import pubmed_client as pc


def pytest_configure(config: pytest.Config) -> None:
    """Configure pytest with custom markers."""
    config.addinivalue_line(
        "markers", "integration: mark test as integration test (requires network)"
    )
    config.addinivalue_line("markers", "slow: mark test as slow running")


@pytest.fixture(scope="session")
def pubmed_client() -> pc.PubMedClient:
    """Create a basic PubMed client for testing."""
    return pc.PubMedClient()


@pytest.fixture(scope="session")
def pmc_client() -> pc.PmcClient:
    """Create a basic PMC client for testing."""
    return pc.PmcClient()


@pytest.fixture(scope="session")
def combined_client() -> pc.Client:
    """Create a basic combined client for testing."""
    return pc.Client()
