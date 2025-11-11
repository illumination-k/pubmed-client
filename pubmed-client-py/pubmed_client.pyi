from collections.abc import Sequence

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
    def __init__(self) -> None: ...
    def query(self, term: str | None) -> SearchQuery: ...
    def terms(self, terms: Sequence[str | None] | None) -> SearchQuery: ...
    def limit(self, limit: int | None) -> SearchQuery: ...
    def build(self) -> str: ...

    # Date filtering methods
    def published_in_year(self, year: int) -> SearchQuery: ...
    def published_between(self, start_year: int, end_year: int | None = None) -> SearchQuery: ...
    def published_after(self, year: int) -> SearchQuery: ...
    def published_before(self, year: int) -> SearchQuery: ...

    # Article type filtering methods
    def article_type(self, type_name: str) -> SearchQuery: ...
    def article_types(self, types: Sequence[str]) -> SearchQuery: ...

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
