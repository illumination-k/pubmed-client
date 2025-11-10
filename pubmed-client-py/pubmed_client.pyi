__version__: str

# ================================================================================================
# PubMed Data Models
# ================================================================================================

class Affiliation:
    institution: str | None
    department: str | None
    address: str | None
    country: str | None
    email: str | None

class Author:
    last_name: str | None
    fore_name: str | None
    first_name: str | None
    middle_name: str | None
    initials: str | None
    suffix: str | None
    full_name: str
    orcid: str | None
    is_corresponding: bool

    def affiliations(self) -> list[Affiliation]: ...

class PubMedArticle:
    pmid: str
    title: str
    journal: str
    pub_date: str
    doi: str | None
    pmc_id: str | None
    abstract_text: str | None
    author_count: int

    def authors(self) -> list[Author]: ...
    def article_types(self) -> list[str]: ...
    def keywords(self) -> list[str] | None: ...

class RelatedArticles:
    source_pmids: list[int]
    related_pmids: list[int]
    link_type: str

    def __len__(self) -> int: ...

class PmcLinks:
    source_pmids: list[int]
    pmc_ids: list[str]

    def __len__(self) -> int: ...

class Citations:
    source_pmids: list[int]
    citing_pmids: list[int]

    def __len__(self) -> int: ...

class DatabaseInfo:
    name: str
    menu_name: str
    description: str
    build: str | None
    count: int | None
    last_update: str | None

# ================================================================================================
# PMC Data Models
# ================================================================================================

class PmcAffiliation:
    id: str | None
    institution: str
    department: str | None
    address: str | None
    country: str | None

class PmcAuthor:
    given_names: str | None
    surname: str | None
    full_name: str
    orcid: str | None
    email: str | None
    is_corresponding: bool

    def affiliations(self) -> list[PmcAffiliation]: ...

class Figure:
    id: str
    label: str | None
    caption: str
    alt_text: str | None
    fig_type: str | None
    file_path: str | None
    file_name: str | None

class Table:
    id: str
    label: str | None
    caption: str

class Reference:
    id: str
    title: str | None
    journal: str | None
    year: str | None
    pmid: str | None
    doi: str | None

class ArticleSection:
    title: str | None
    content: str
    section_type: str | None

class PmcFullText:
    pmcid: str
    pmid: str | None
    title: str
    doi: str | None

    def authors(self) -> list[PmcAuthor]: ...
    def sections(self) -> list[ArticleSection]: ...
    def figures(self) -> list[Figure]: ...
    def tables(self) -> list[Table]: ...
    def references(self) -> list[Reference]: ...
    def to_markdown(self) -> str: ...

# ================================================================================================
# Configuration
# ================================================================================================

class ClientConfig:
    def __init__(self) -> None: ...
    def with_api_key(self, api_key: str) -> ClientConfig: ...
    def with_email(self, email: str) -> ClientConfig: ...
    def with_tool(self, tool: str) -> ClientConfig: ...
    def with_rate_limit(self, rate_limit: float) -> ClientConfig: ...
    def with_timeout_seconds(self, timeout_seconds: int) -> ClientConfig: ...
    def with_cache(self) -> ClientConfig: ...

# ================================================================================================
# Client Implementations
# ================================================================================================

class PubMedClient:
    def __init__(self) -> None: ...
    @staticmethod
    def with_config(config: ClientConfig) -> PubMedClient: ...
    def search_articles(self, query: str, limit: int) -> list[str]: ...
    def search_and_fetch(self, query: str, limit: int) -> list[PubMedArticle]: ...
    def fetch_article(self, pmid: str) -> PubMedArticle: ...
    def get_database_list(self) -> list[str]: ...
    def get_database_info(self, database: str) -> DatabaseInfo: ...
    def get_related_articles(self, pmids: list[int]) -> RelatedArticles: ...
    def get_pmc_links(self, pmids: list[int]) -> PmcLinks: ...
    def get_citations(self, pmids: list[int]) -> Citations: ...

class PmcClient:
    def __init__(self) -> None: ...
    @staticmethod
    def with_config(config: ClientConfig) -> PmcClient: ...
    def fetch_full_text(self, pmcid: str) -> PmcFullText: ...
    def check_pmc_availability(self, pmid: str) -> str | None: ...

class Client:
    pubmed: PubMedClient
    pmc: PmcClient

    def __init__(self) -> None: ...
    @staticmethod
    def with_config(config: ClientConfig) -> Client: ...

# ================================================================================================
# Query Builder
# ================================================================================================

class SearchQuery:
    """
    Builder for constructing PubMed search queries.

    Provides a fluent API for building queries programmatically instead of
    writing raw query strings. Supports method chaining for clean query construction.

    Examples:
        >>> query = SearchQuery().query("covid-19").limit(10)
        >>> query_string = query.build()
        >>> print(query_string)
        covid-19
    """

    def __init__(self) -> None:
        """Create a new empty search query builder."""
        ...

    def query(self, term: str | None) -> SearchQuery:
        """
        Add a search term to the query.

        Terms are accumulated (not replaced) and will be space-separated in the final query.
        None and empty strings are silently filtered out.

        Args:
            term: Search term string. None and empty strings are silently filtered.

        Returns:
            Self for method chaining.

        Example:
            >>> query = SearchQuery().query("covid-19").query("treatment")
            >>> query.build()
            'covid-19 treatment'
        """
        ...

    def terms(self, terms: list[str] | None) -> SearchQuery:
        """
        Add multiple search terms to the query.

        Each term is processed like query(). None items and empty strings are filtered.

        Args:
            terms: List of search term strings. None items and empty strings are filtered.

        Returns:
            Self for method chaining.

        Example:
            >>> query = SearchQuery().terms(["covid-19", "vaccine", "efficacy"])
            >>> query.build()
            'covid-19 vaccine efficacy'
        """
        ...

    def limit(self, limit: int | None) -> SearchQuery:
        """
        Set the maximum number of results to return.

        Validates that limit is >0 and ≤10,000. None is treated as "use default" (20).

        Args:
            limit: Maximum number of results. None means use default (20).

        Returns:
            Self for method chaining.

        Raises:
            ValueError: If limit <= 0 or limit > 10,000.

        Example:
            >>> query = SearchQuery().query("cancer").limit(50)
        """
        ...

    def build(self) -> str:
        """
        Build the final PubMed query string.

        Terms are joined with space separators (PubMed's default OR logic).

        Returns:
            Query string for PubMed E-utilities API.

        Raises:
            ValueError: If no search terms have been added.

        Example:
            >>> query = SearchQuery().query("covid-19").query("treatment")
            >>> query.build()
            'covid-19 treatment'
        """
        ...

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
    "SearchQuery",
    "Table",
]
