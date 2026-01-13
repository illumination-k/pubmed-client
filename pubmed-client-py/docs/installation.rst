Installation
============

Requirements
------------

- Python 3.12 or later
- pip or uv package manager

Installing from PyPI
--------------------

The easiest way to install pubmed-client-py is using pip:

.. code-block:: bash

    pip install pubmed-client-py

Or using uv (recommended for development):

.. code-block:: bash

    uv add pubmed-client-py

Installing from Source
----------------------

To install from source, you'll need:

- Rust toolchain (https://rustup.rs/)
- maturin build tool

1. Clone the repository:

.. code-block:: bash

    git clone https://github.com/illumination-k/pubmed-client-rs.git
    cd pubmed-client-rs/pubmed-client-py

2. Build and install with maturin:

.. code-block:: bash

    # Using uv (recommended)
    uv run --with maturin maturin develop

    # Or using pip
    pip install maturin
    maturin develop

Development Installation
------------------------

For development with all dependencies:

.. code-block:: bash

    cd pubmed-client-py
    uv sync --group dev

This installs the package in development mode along with:

- pytest and pytest-cov for testing
- mypy for type checking
- ruff for linting and formatting

Verifying Installation
----------------------

To verify the installation:

.. code-block:: python

    import pubmed_client
    print(pubmed_client.__version__)

    # Test basic functionality
    client = pubmed_client.Client()
    articles = client.pubmed.search_and_fetch("covid-19", 5)
    print(f"Found {len(articles)} articles")

NCBI API Key (Optional)
-----------------------

While not required, setting up an NCBI API key increases your rate limit from 3 to 10 requests per second.

1. Create an account at https://www.ncbi.nlm.nih.gov/account/
2. Get your API key from https://www.ncbi.nlm.nih.gov/account/settings/
3. Use it with the client:

.. code-block:: python

    config = pubmed_client.ClientConfig()\\
        .with_api_key("your_api_key_here")\\
        .with_email("your_email@example.com")

    client = pubmed_client.Client.with_config(config)

Setting your email is also recommended by NCBI for identification purposes.
