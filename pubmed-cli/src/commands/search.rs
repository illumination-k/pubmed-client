use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use pubmed_client::pubmed::ArticleType;
use serde_json;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct Search {
    /// Search query (free text)
    #[arg(value_name = "QUERY")]
    query: Option<String>,

    /// Maximum number of results to return
    #[arg(short, long, default_value = "10")]
    limit: usize,

    /// Save results to file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Show only PMIDs (one per line)
    #[arg(long)]
    ids_only: bool,

    // Date filters
    /// Filter articles published from this year onwards
    #[arg(long)]
    from_year: Option<u32>,

    /// Filter articles published up to this year
    #[arg(long)]
    to_year: Option<u32>,

    /// Filter articles published in a specific year
    #[arg(long)]
    year: Option<u32>,

    /// Filter by author name
    #[arg(long)]
    author: Option<String>,

    /// Filter by first author
    #[arg(long)]
    first_author: Option<String>,

    /// Filter by last author
    #[arg(long)]
    last_author: Option<String>,

    /// Filter by journal name
    #[arg(long)]
    journal: Option<String>,

    /// Filter by journal abbreviation
    #[arg(long)]
    journal_abbrev: Option<String>,

    // MeSH terms
    /// Filter by MeSH term
    #[arg(long)]
    mesh_term: Option<String>,

    /// Filter by MeSH major topic
    #[arg(long)]
    mesh_major: Option<String>,

    /// Filter by organism (uses scientific or common name)
    #[arg(long)]
    organism: Option<String>,

    // Article type and content filters
    /// Filter by article type
    #[arg(long, value_enum)]
    article_type: Option<ArticleTypeArg>,

    /// Include only open access articles
    #[arg(long)]
    open_access: bool,

    // Other identifiers
    /// Filter by grant number
    #[arg(long)]
    grant_number: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ArticleTypeArg {
    ClinicalTrial,
    Review,
    SystematicReview,
    MetaAnalysis,
    CaseReport,
    RandomizedControlledTrial,
    ObservationalStudy,
}

impl From<ArticleTypeArg> for ArticleType {
    fn from(arg: ArticleTypeArg) -> Self {
        match arg {
            ArticleTypeArg::ClinicalTrial => ArticleType::ClinicalTrial,
            ArticleTypeArg::Review => ArticleType::Review,
            ArticleTypeArg::SystematicReview => ArticleType::SystematicReview,
            ArticleTypeArg::MetaAnalysis => ArticleType::MetaAnalysis,
            ArticleTypeArg::CaseReport => ArticleType::CaseReport,
            ArticleTypeArg::RandomizedControlledTrial => ArticleType::RandomizedControlledTrial,
            ArticleTypeArg::ObservationalStudy => ArticleType::ObservationalStudy,
        }
    }
}

impl Search {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        // Build the search query
        let search_query = self.build_query()?;

        if self.ids_only {
            // Just search for PMIDs
            let pmids = client.search_articles(&search_query, self.limit).await?;
            let output = pmids.join("\n");
            self.output_results(&output).await?;
        } else {
            // Fetch full articles
            let articles = client.search_and_fetch(&search_query, self.limit).await?;
            let output = serde_json::to_string_pretty(&articles)?;
            self.output_results(&output).await?;
        }

        Ok(())
    }

    fn build_query(&self) -> Result<String> {
        let mut query = pubmed_client::pubmed::SearchQuery::new();

        // Add main query if provided
        if let Some(ref q) = self.query {
            query = query.query(q);
        }

        // Date filters
        if let Some(year) = self.year {
            query = query.published_in_year(year);
        } else {
            match (self.from_year, self.to_year) {
                (Some(from), Some(to)) => {
                    query = query.date_range(from, Some(to));
                }
                (Some(from), None) => {
                    query = query.published_after(from);
                }
                (None, Some(to)) => {
                    query = query.published_before(to);
                }
                (None, None) => {}
            }
        }

        if let Some(ref author) = self.author {
            query = query.author(author);
        }

        if let Some(ref first_author) = self.first_author {
            query = query.first_author(first_author);
        }

        if let Some(ref last_author) = self.last_author {
            query = query.last_author(last_author);
        }

        if let Some(ref journal) = self.journal {
            query = query.journal(journal);
        }

        if let Some(ref journal_abbrev) = self.journal_abbrev {
            query = query.journal_abbreviation(journal_abbrev);
        }

        // MeSH terms
        if let Some(ref mesh_term) = self.mesh_term {
            query = query.mesh_term(mesh_term);
        }

        if let Some(ref mesh_major) = self.mesh_major {
            query = query.mesh_major_topic(mesh_major);
        }

        // Organism filter
        if let Some(ref organism) = self.organism {
            query = query.organism_mesh(organism);
        }

        // Article type and content filters
        if let Some(ref article_type) = self.article_type {
            let article_type: ArticleType = article_type.clone().into();
            query = query.article_type(article_type);
        }

        if self.open_access {
            query = query.free_full_text_only();
        }

        // Other identifiers
        if let Some(ref grant_number) = self.grant_number {
            query = query.grant_number(grant_number);
        }

        // Set limit
        query = query.limit(self.limit);

        Ok(query.build())
    }

    async fn output_results(&self, content: &str) -> Result<()> {
        match &self.output {
            Some(path) => {
                tokio::fs::write(path, content).await?;
                tracing::info!(path = %path.display(), "Results saved to file");
            }
            None => {
                println!("{}", content);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_basic() {
        let search = Search {
            query: Some("covid-19".to_string()),
            limit: 10,
            output: None,
            ids_only: false,
            from_year: None,
            to_year: None,
            year: None,
            author: None,
            first_author: None,
            last_author: None,
            journal: None,
            journal_abbrev: None,
            mesh_term: None,
            mesh_major: None,
            organism: None,
            article_type: None,
            open_access: false,
            grant_number: None,
        };

        let query = search.build_query().unwrap();
        assert_eq!(query, "covid-19");
    }

    #[test]
    fn test_build_query_with_filters() {
        let search = Search {
            query: Some("cancer".to_string()),
            limit: 5,
            output: None,
            ids_only: false,
            from_year: Some(2020),
            to_year: Some(2023),
            year: None,
            author: Some("Smith".to_string()),
            first_author: None,
            last_author: None,
            journal: Some("Nature".to_string()),
            journal_abbrev: None,
            mesh_term: None,
            mesh_major: None,
            organism: None,
            article_type: Some(ArticleTypeArg::Review),
            open_access: true,
            grant_number: None,
        };

        let query = search.build_query().unwrap();
        assert!(query.contains("cancer"));
        assert!(query.contains("2020:2023[pdat]"));
        assert!(query.contains("Smith[au]"));
        assert!(query.contains("Nature[ta]"));
        assert!(query.contains("Review[pt]"));
        assert!(query.contains("free full text[sb]"));
    }

    #[test]
    fn test_build_query_with_organism() {
        let search = Search {
            query: Some("gene expression".to_string()),
            limit: 10,
            output: None,
            ids_only: false,
            from_year: None,
            to_year: None,
            year: None,
            author: None,
            first_author: None,
            last_author: None,
            journal: None,
            journal_abbrev: None,
            mesh_term: None,
            mesh_major: None,
            organism: Some("Mus musculus".to_string()),
            article_type: None,
            open_access: false,
            grant_number: None,
        };

        let query = search.build_query().unwrap();
        assert!(query.contains("gene expression"));
        assert!(query.contains("Mus musculus[mh]"));
    }

    #[test]
    fn test_article_type_conversion() {
        let clinical_trial: ArticleType = ArticleTypeArg::ClinicalTrial.into();
        assert_eq!(clinical_trial, ArticleType::ClinicalTrial);

        let review: ArticleType = ArticleTypeArg::Review.into();
        assert_eq!(review, ArticleType::Review);
    }

    #[test]
    fn test_ids_only_mode() {
        let search = Search {
            query: Some("test".to_string()),
            limit: 10,
            output: None,
            ids_only: true,
            from_year: None,
            to_year: None,
            year: None,
            author: None,
            first_author: None,
            last_author: None,
            journal: None,
            journal_abbrev: None,
            mesh_term: None,
            mesh_major: None,
            organism: None,
            article_type: None,
            open_access: false,
            grant_number: None,
        };

        // We can't test the actual execution here since it requires network calls,
        // but we can verify that the search struct is properly constructed
        assert!(search.ids_only);
        assert_eq!(search.query, Some("test".to_string()));
    }
}
