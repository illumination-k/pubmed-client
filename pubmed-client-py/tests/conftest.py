"""Pytest configuration and shared fixtures."""

import pytest


def pytest_configure(config: pytest.Config) -> None:
    """Configure pytest with custom markers."""
    config.addinivalue_line(
        "markers", "integration: mark test as integration test (requires network)"
    )
    config.addinivalue_line("markers", "slow: mark test as slow running")
