// TypeScript definitions for PubMed Client WebAssembly

export interface WasmClientConfig {
    new(): WasmClientConfig;
    set_api_key(api_key: string): void;
    set_email(email: string): void;
    set_tool(tool: string): void;
    set_rate_limit(rate_limit: number): void;
    set_timeout_seconds(timeout_seconds: number): void;
}

export interface Article {
    pmid: string;
    title: string;
    authors: string[];
    journal: string;
    pub_date: string;
    abstract_text?: string;
    doi?: string;
    article_types: string[];
}

export interface Author {
    given_names?: string;
    surname?: string;
    full_name: string;
    email?: string;
    affiliations: string[];
    is_corresponding: boolean;
}

export interface Journal {
    title: string;
    abbreviation?: string;
    publisher?: string;
    issn_print?: string;
    issn_electronic?: string;
}

export interface Section {
    section_type: string;
    title?: string;
    content: string;
}

export interface Reference {
    id: string;
    title?: string;
    authors: string[];
    journal?: string;
    year?: string;
    pmid?: string;
    doi?: string;
}

export interface FullText {
    pmcid: string;
    pmid?: string;
    title: string;
    authors: Author[];
    journal: Journal;
    pub_date: string;
    doi?: string;
    sections: Section[];
    references: Reference[];
    article_type?: string;
    keywords: string[];
}

export interface RelatedArticles {
    source_pmids: number[];
    related_pmids: number[];
    total_count: number;
}

export interface PmcLinks {
    source_pmids: number[];
    pmc_ids: string[];
}

export interface Citations {
    source_pmids: number[];
    citing_pmids: number[];
    total_count: number;
}

export interface WasmPubMedClient {
    new(): WasmPubMedClient;
    with_config(config: WasmClientConfig): WasmPubMedClient;

    search_articles(query: string, limit: number): Promise<Article[]>;
    fetch_article(pmid: string): Promise<Article>;
    fetch_full_text(pmcid: string): Promise<FullText>;
    check_pmc_availability(pmid: string): Promise<string | null>;
    convert_to_markdown(full_text: FullText): string;
    get_related_articles(pmids: number[]): Promise<RelatedArticles>;
}

export declare const WasmClientConfig: {
    new(): WasmClientConfig;
};

export declare const WasmPubMedClient: {
    new(): WasmPubMedClient;
    with_config(config: WasmClientConfig): WasmPubMedClient;
};

// Re-export for convenience
export { WasmClientConfig as ClientConfig };
export { WasmPubMedClient as PubMedClient };
