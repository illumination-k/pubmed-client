"""Tests for ClientConfig."""

import pubmed_client


def test_config_creation() -> None:
    """Test creating a default configuration."""
    config = pubmed_client.ClientConfig()
    assert config is not None
    assert repr(config) == "ClientConfig(...)"


def test_config_with_api_key() -> None:
    """Test setting API key."""
    config = pubmed_client.ClientConfig()
    config.with_api_key("test_api_key_123")
    assert config is not None


def test_config_with_email() -> None:
    """Test setting email."""
    config = pubmed_client.ClientConfig()
    config.with_email("test@example.com")
    assert config is not None


def test_config_with_tool() -> None:
    """Test setting tool name."""
    config = pubmed_client.ClientConfig()
    config.with_tool("test-tool")
    assert config is not None


def test_config_with_rate_limit() -> None:
    """Test setting custom rate limit."""
    config = pubmed_client.ClientConfig()
    config.with_rate_limit(5.0)
    assert config is not None


def test_config_with_timeout() -> None:
    """Test setting timeout."""
    config = pubmed_client.ClientConfig()
    config.with_timeout_seconds(60)
    assert config is not None


def test_config_with_cache() -> None:
    """Test enabling cache."""
    config = pubmed_client.ClientConfig()
    config.with_cache()
    assert config is not None


def test_config_builder_pattern() -> None:
    """Test chaining configuration methods."""
    config = pubmed_client.ClientConfig()
    result = (
        config.with_api_key("test_key")
        .with_email("test@example.com")
        .with_tool("pytest")
        .with_rate_limit(3.0)
        .with_timeout_seconds(30)
        .with_cache()
    )
    assert result is not None
