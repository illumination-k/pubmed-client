"""
Type stubs for pubmed_client

Python bindings for PubMed and PMC API client library.
"""

from typing import Optional

__version__: str

# ================================================================================================
# PubMed Data Models
# ================================================================================================

class Affiliation:
    """Author affiliation information from PubMed."""

    institution: Optional[str]
    department: Optional[str]
    address: Optional[str]
    country: Optional[str]
    email: Optional[str]

    def __repr__(self) -> str: ...

class Author:
    """Author information from PubMed article."""

    last_name: Optional[str]
    fore_name: Optional[str]
    first_name: Optional[str]
    middle_name: Optional[str]
    initials: Optional[str]
    suffix: Optional[str]
    full_name: str
    orcid: Optional[str]
    is_corresponding: bool

    def affiliations(self) -> list[Affiliation]:
        """Get list of author affiliations."""
        ...

    def __repr__(self) -> str: ...

class PubMedArticle:
    """PubMed article metadata."""

    pmid: str
    title: str
    journal: str
    pub_date: str
    doi: Optional[str]
    abstract_text: Optional[str]
    author_count: int

    def authors(self) -> list[Author]:
        """Get list of article authors."""
        ...

    def article_types(self) -> list[str]:
        """Get article type classifications."""
        ...

    def keywords(self) -> Optional[list[str]]:
        """Get article keywords if available."""
        ...

    def __repr__(self) -> str: ...

class RelatedArticles:
    """Related articles from ELink API."""

    source_pmids: list[int]
    related_pmids: list[int]
    link_type: str

    def __repr__(self) -> str: ...
    def __len__(self) -> int: ...

class PmcLinks:
    """PMC full-text availability links."""

    source_pmids: list[int]
    pmc_ids: list[str]

    def __repr__(self) -> str: ...
    def __len__(self) -> int: ...

class Citations:
    """Citation information from ELink API."""

    source_pmids: list[int]
    citing_pmids: list[int]

    def __repr__(self) -> str: ...
    def __len__(self) -> int: ...

class DatabaseInfo:
    """NCBI database information from EInfo API."""

    name: str
    menu_name: str
    description: str
    build: Optional[str]
    count: Optional[int]
    last_update: Optional[str]

    def __repr__(self) -> str: ...

# ================================================================================================
# PMC Data Models
# ================================================================================================

class PmcAffiliation:
    """Author affiliation information from PMC."""

    id: Optional[str]
    institution: str
    department: Optional[str]
    address: Optional[str]
    country: Optional[str]

    def __repr__(self) -> str: ...

class PmcAuthor:
    """Author information from PMC article."""

    given_names: Optional[str]
    surname: Optional[str]
    full_name: str
    orcid: Optional[str]
    email: Optional[str]
    is_corresponding: bool

    def affiliations(self) -> list[PmcAffiliation]:
        """Get list of author affiliations."""
        ...

    def __repr__(self) -> str: ...

class Figure:
    """Figure from PMC article."""

    id: str
    label: Optional[str]
    caption: str
    alt_text: Optional[str]
    fig_type: Optional[str]
    file_path: Optional[str]
    file_name: Optional[str]

    def __repr__(self) -> str: ...

class Table:
    """Table from PMC article."""

    id: str
    label: Optional[str]
    caption: str

    def __repr__(self) -> str: ...

class Reference:
    """Reference from PMC article."""

    id: str
    title: Optional[str]
    journal: Optional[str]
    year: Optional[str]
    pmid: Optional[str]
    doi: Optional[str]

    def __repr__(self) -> str: ...

class ArticleSection:
    """Section from PMC article."""

    title: Optional[str]
    content: str
    section_type: Optional[str]

    def __repr__(self) -> str: ...

class PmcFullText:
    """PMC full-text article with structured content."""

    pmcid: str
    pmid: Optional[str]
    title: str
    doi: Optional[str]

    def authors(self) -> list[PmcAuthor]:
        """Get list of article authors."""
        ...

    def sections(self) -> list[ArticleSection]:
        """Get list of article sections."""
        ...

    def figures(self) -> list[Figure]:
        """Get list of all figures from all sections."""
        ...

    def tables(self) -> list[Table]:
        """Get list of all tables from all sections."""
        ...

    def references(self) -> list[Reference]:
        """Get list of references."""
        ...

    def __repr__(self) -> str: ...

# ================================================================================================
# Configuration
# ================================================================================================

class ClientConfig:
    """
    Configuration for PubMed and PMC clients.

    Examples:
        >>> config = ClientConfig()
        >>> config.with_api_key("your_api_key").with_email("you@example.com")
        >>> client = Client.with_config(config)
    """

    def __init__(self) -> None:
        """Create a new configuration with default settings."""
        ...

    def with_api_key(self, api_key: str) -> ClientConfig:
        """
        Set the NCBI API key for increased rate limits (10 req/sec instead of 3).

        Args:
            api_key: Your NCBI API key

        Returns:
            Self for method chaining
        """
        ...

    def with_email(self, email: str) -> ClientConfig:
        """
        Set the email address for identification (recommended by NCBI).

        Args:
            email: Contact email address

        Returns:
            Self for method chaining
        """
        ...

    def with_tool(self, tool: str) -> ClientConfig:
        """
        Set the tool name for identification.

        Args:
            tool: Application or tool name (default: "pubmed-client-py")

        Returns:
            Self for method chaining
        """
        ...

    def with_rate_limit(self, rate_limit: float) -> ClientConfig:
        """
        Set custom rate limit in requests per second.

        Args:
            rate_limit: Requests per second (e.g., 3.0, 10.0)

        Returns:
            Self for method chaining
        """
        ...

    def with_timeout_seconds(self, timeout_seconds: int) -> ClientConfig:
        """
        Set HTTP request timeout in seconds.

        Args:
            timeout_seconds: Timeout in seconds

        Returns:
            Self for method chaining
        """
        ...

    def with_cache(self) -> ClientConfig:
        """
        Enable default response caching.

        Returns:
            Self for method chaining
        """
        ...

    def __repr__(self) -> str: ...

# ================================================================================================
# Client Implementations
# ================================================================================================

class PubMedClient:
    """
    PubMed client for searching and fetching article metadata.

    Examples:
        >>> client = PubMedClient()
        >>> articles = client.search_and_fetch("covid-19", 10)
        >>> article = client.fetch_article("31978945")
    """

    def __init__(self) -> None:
        """Create a new PubMed client with default configuration."""
        ...

    @staticmethod
    def with_config(config: ClientConfig) -> PubMedClient:
        """
        Create a new PubMed client with custom configuration.

        Args:
            config: Client configuration

        Returns:
            Configured PubMed client
        """
        ...

    def search_and_fetch(self, query: str, limit: int) -> list[PubMedArticle]:
        """
        Search for articles and fetch their metadata.

        Args:
            query: Search query string
            limit: Maximum number of articles to return

        Returns:
            List of PubMedArticle objects
        """
        ...

    def fetch_article(self, pmid: str) -> PubMedArticle:
        """
        Fetch a single article by PMID.

        Args:
            pmid: PubMed ID as a string

        Returns:
            PubMedArticle object
        """
        ...

    def get_database_list(self) -> list[str]:
        """
        Get list of all available NCBI databases.

        Returns:
            List of database names
        """
        ...

    def get_database_info(self, database: str) -> DatabaseInfo:
        """
        Get detailed information about a specific database.

        Args:
            database: Database name (e.g., "pubmed", "pmc")

        Returns:
            DatabaseInfo object
        """
        ...

    def get_related_articles(self, pmids: list[int]) -> RelatedArticles:
        """
        Get related articles for given PMIDs.

        Args:
            pmids: List of PubMed IDs

        Returns:
            RelatedArticles object
        """
        ...

    def get_pmc_links(self, pmids: list[int]) -> PmcLinks:
        """
        Get PMC links for given PMIDs (full-text availability).

        Args:
            pmids: List of PubMed IDs

        Returns:
            PmcLinks object containing available PMC IDs
        """
        ...

    def get_citations(self, pmids: list[int]) -> Citations:
        """
        Get citing articles for given PMIDs.

        Args:
            pmids: List of PubMed IDs

        Returns:
            Citations object containing citing article PMIDs
        """
        ...

    def __repr__(self) -> str: ...

class PmcClient:
    """
    PMC client for fetching full-text articles.

    Examples:
        >>> client = PmcClient()
        >>> full_text = client.fetch_full_text("PMC7906746")
        >>> pmcid = client.check_pmc_availability("31978945")
    """

    def __init__(self) -> None:
        """Create a new PMC client with default configuration."""
        ...

    @staticmethod
    def with_config(config: ClientConfig) -> PmcClient:
        """
        Create a new PMC client with custom configuration.

        Args:
            config: Client configuration

        Returns:
            Configured PMC client
        """
        ...

    def fetch_full_text(self, pmcid: str) -> PmcFullText:
        """
        Fetch full text article from PMC.

        Args:
            pmcid: PMC ID (e.g., "PMC7906746")

        Returns:
            PmcFullText object containing structured article content
        """
        ...

    def check_pmc_availability(self, pmid: str) -> Optional[str]:
        """
        Check if PMC full text is available for a PMID.

        Args:
            pmid: PubMed ID as a string

        Returns:
            PMC ID if available, None otherwise
        """
        ...

    def __repr__(self) -> str: ...

class Client:
    """
    Combined client with both PubMed and PMC functionality.

    This is the main client you'll typically use. It provides access to both
    PubMed metadata searches and PMC full-text retrieval.

    Examples:
        >>> client = Client()
        >>> # Access PubMed client
        >>> articles = client.pubmed.search_and_fetch("covid-19", 10)
        >>> # Access PMC client
        >>> full_text = client.pmc.fetch_full_text("PMC7906746")
    """

    pubmed: PubMedClient
    pmc: PmcClient

    def __init__(self) -> None:
        """Create a new combined client with default configuration."""
        ...

    @staticmethod
    def with_config(config: ClientConfig) -> Client:
        """
        Create a new combined client with custom configuration.

        Args:
            config: Client configuration

        Returns:
            Configured combined client
        """
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "Affiliation",
    "ArticleSection",
    "Author",
    "Citations",
    "Client",
    "ClientConfig",
    "DatabaseInfo",
    "Figure",
    "PmcAffiliation",
    "PmcAuthor",
    "PmcClient",
    "PmcFullText",
    "PmcLinks",
    "PubMedArticle",
    "PubMedClient",
    "Reference",
    "RelatedArticles",
    "Table",
]
