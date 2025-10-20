# Python Bindings Test Suite

This directory contains the test suite for the `pubmed-client` Python bindings.

## Test Organization

- `test_config.py` - Configuration and builder pattern tests
- `test_client.py` - Client initialization and basic functionality tests
- `test_models.py` - Data model tests (requires network access)
- `test_integration.py` - Integration tests with real API calls (requires network access)
- `conftest.py` - Pytest configuration and shared fixtures

## Running Tests

### Prerequisites

1. Build the Python package:
   ```bash
   cd pubmed-client-py
   uv run maturin develop
   ```

2. Install test dependencies:
   ```bash
   uv sync --group dev
   ```

### Run All Tests (Excluding Integration Tests)

```bash
uv run pytest -m "not integration"
```

### Run Unit Tests Only

```bash
uv run pytest test_config.py test_client.py
```

### Run Integration Tests

Integration tests make real API calls to NCBI and require network access.

```bash
# Run all integration tests
uv run pytest -m integration

# Run integration tests with verbose output
uv run pytest -m integration -v

# Run specific integration test file
uv run pytest tests/test_integration.py
```

### Run All Tests (Including Integration)

```bash
uv run pytest
```

### Using an API Key

For higher rate limits during integration tests, set the `NCBI_API_KEY` environment variable:

```bash
export NCBI_API_KEY="your_api_key_here"
uv run pytest -m integration
```

## Test Markers

- `@pytest.mark.integration` - Tests that require network access to NCBI APIs
- `@pytest.mark.slow` - Tests that take a long time to run

### Running Tests by Marker

```bash
# Run only integration tests
uv run pytest -m integration

# Exclude integration tests
uv run pytest -m "not integration"

# Run slow tests
uv run pytest -m slow

# Exclude slow tests
uv run pytest -m "not slow"
```

## Test Coverage

Generate test coverage reports:

```bash
# Run tests with coverage
uv run pytest --cov=pubmed_client --cov-report=html

# Open coverage report in browser
open htmlcov/index.html
```

## Continuous Integration

In CI environments, run tests without integration tests to avoid overwhelming NCBI servers:

```bash
uv run pytest -m "not integration"
```

## Troubleshooting

### Import Errors

If you get import errors, make sure the package is built:

```bash
uv run maturin develop
```

### Rate Limiting Errors

If you encounter rate limiting errors (HTTP 429), try:

1. Using an API key (see above)
2. Running fewer tests at once
3. Increasing the delay between tests

### Network Errors

Integration tests require network access. If you're offline or behind a firewall, skip them:

```bash
uv run pytest -m "not integration"
```

## Writing New Tests

### Unit Tests

Place unit tests in files named `test_*.py`. They should not require network access.

```python
def test_config_creation() -> None:
    """Test creating a configuration."""
    import pubmed_client

    config = pubmed_client.ClientConfig()
    assert config is not None
```

### Integration Tests

Mark integration tests with `@pytest.mark.integration`:

```python
@pytest.mark.integration
def test_fetch_article(client) -> None:
    """Test fetching an article from PubMed."""
    article = client.pubmed.fetch_article("31978945")
    assert article is not None
```

### Fixtures

Use fixtures from `conftest.py` for common test setup:

```python
def test_example(pubmed_client) -> None:
    """Test using a shared fixture."""
    assert pubmed_client is not None
```

## Best Practices

1. **Use descriptive test names** - Test names should clearly describe what they test
2. **Mark integration tests** - Always mark tests that require network access
3. **Use conservative rate limits** - Set rate_limit to 1.0 or lower in tests
4. **Check for None** - Handle cases where API data might not be available
5. **Test both success and failure** - Include error handling tests
6. **Use real PMIDs** - Use well-known PMIDs like 31978945 for reproducible tests
