//! Advanced search methods for MeSH terms, authors, and specialized filtering

use super::SearchQuery;

impl SearchQuery {
    /// Filter by MeSH major topic
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by as a major topic
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_major_topic("Diabetes Mellitus, Type 2");
    /// ```
    pub fn mesh_major_topic<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Major Topic]", mesh_term.into()));
        self
    }

    /// Filter by MeSH term
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Neoplasms");
    /// ```
    pub fn mesh_term<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Terms]", mesh_term.into()));
        self
    }

    /// Filter by multiple MeSH terms
    ///
    /// # Arguments
    ///
    /// * `mesh_terms` - MeSH terms to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_terms(&["Neoplasms", "Antineoplastic Agents"]);
    /// ```
    pub fn mesh_terms<S: AsRef<str>>(mut self, mesh_terms: &[S]) -> Self {
        for term in mesh_terms {
            self = self.mesh_term(term.as_ref());
        }
        self
    }

    /// Filter by MeSH subheading
    ///
    /// # Arguments
    ///
    /// * `subheading` - MeSH subheading to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Diabetes Mellitus")
    ///     .mesh_subheading("drug therapy");
    /// ```
    pub fn mesh_subheading<S: Into<String>>(mut self, subheading: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Subheading]", subheading.into()));
        self
    }

    /// Filter by first author
    ///
    /// # Arguments
    ///
    /// * `author` - First author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer treatment")
    ///     .first_author("Smith J");
    /// ```
    pub fn first_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters
            .push(format!("{}[First Author]", author.into()));
        self
    }

    /// Filter by last author
    ///
    /// # Arguments
    ///
    /// * `author` - Last author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genomics")
    ///     .last_author("Johnson M");
    /// ```
    pub fn last_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Last Author]", author.into()));
        self
    }

    /// Filter by any author
    ///
    /// # Arguments
    ///
    /// * `author` - Author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .author("Williams K");
    /// ```
    pub fn author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Author]", author.into()));
        self
    }

    /// Filter by institution/affiliation
    ///
    /// # Arguments
    ///
    /// * `institution` - Institution name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cardiology research")
    ///     .affiliation("Harvard Medical School");
    /// ```
    pub fn affiliation<S: Into<String>>(mut self, institution: S) -> Self {
        self.filters
            .push(format!("{}[Affiliation]", institution.into()));
        self
    }

    /// Filter by ORCID identifier
    ///
    /// # Arguments
    ///
    /// * `orcid_id` - ORCID identifier to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("computational biology")
    ///     .orcid("0000-0001-2345-6789");
    /// ```
    pub fn orcid<S: Into<String>>(mut self, orcid_id: S) -> Self {
        self.filters
            .push(format!("{}[Author - Identifier]", orcid_id.into()));
        self
    }

    /// Filter to human studies only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("drug treatment")
    ///     .human_studies_only();
    /// ```
    pub fn human_studies_only(mut self) -> Self {
        self.filters.push("humans[mh]".to_string());
        self
    }

    /// Filter to animal studies only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("preclinical research")
    ///     .animal_studies_only();
    /// ```
    pub fn animal_studies_only(mut self) -> Self {
        self.filters.push("animals[mh]".to_string());
        self
    }

    /// Filter by age group
    ///
    /// # Arguments
    ///
    /// * `age_group` - Age group to filter by (e.g., "Child", "Adult", "Aged")
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("pediatric medicine")
    ///     .age_group("Child");
    /// ```
    pub fn age_group<S: Into<String>>(mut self, age_group: S) -> Self {
        self.filters.push(format!("{}[mh]", age_group.into()));
        self
    }

    /// Add a custom filter
    ///
    /// # Arguments
    ///
    /// * `filter` - Custom filter string in PubMed syntax
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("research")
    ///     .custom_filter("humans[mh]");
    /// ```
    pub fn custom_filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.filters.push(filter.into());
        self
    }
}
