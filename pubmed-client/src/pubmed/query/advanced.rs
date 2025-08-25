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

    /// Filter by organism (scientific or common name)
    ///
    /// # Arguments
    ///
    /// * `organism` - Organism name (scientific or common name)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // Using scientific name
    /// let query = SearchQuery::new()
    ///     .query("gene expression")
    ///     .organism("Mus musculus");
    ///
    /// // Using common name
    /// let query = SearchQuery::new()
    ///     .query("metabolism")
    ///     .organism("mouse");
    ///
    /// // Using bacteria
    /// let query = SearchQuery::new()
    ///     .query("antibiotic resistance")
    ///     .organism("Escherichia coli");
    /// ```
    pub fn organism<S: Into<String>>(mut self, organism: S) -> Self {
        self.filters.push(format!("{}[Organism]", organism.into()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_term() {
        let query = SearchQuery::new().mesh_term("Neoplasms");
        assert_eq!(query.build(), "Neoplasms[MeSH Terms]");
    }

    #[test]
    fn test_mesh_major_topic() {
        let query = SearchQuery::new().mesh_major_topic("Diabetes Mellitus, Type 2");
        assert_eq!(query.build(), "Diabetes Mellitus, Type 2[MeSH Major Topic]");
    }

    #[test]
    fn test_multiple_mesh_terms() {
        let mesh_terms = ["Neoplasms", "Antineoplastic Agents"];
        let query = SearchQuery::new().mesh_terms(&mesh_terms);
        assert_eq!(
            query.build(),
            "Neoplasms[MeSH Terms] AND Antineoplastic Agents[MeSH Terms]"
        );
    }

    #[test]
    fn test_mesh_subheading() {
        let query = SearchQuery::new()
            .mesh_term("Diabetes Mellitus")
            .mesh_subheading("drug therapy");
        assert_eq!(
            query.build(),
            "Diabetes Mellitus[MeSH Terms] AND drug therapy[MeSH Subheading]"
        );
    }

    #[test]
    fn test_first_author() {
        let query = SearchQuery::new().first_author("Smith J");
        assert_eq!(query.build(), "Smith J[First Author]");
    }

    #[test]
    fn test_last_author() {
        let query = SearchQuery::new().last_author("Johnson M");
        assert_eq!(query.build(), "Johnson M[Last Author]");
    }

    #[test]
    fn test_any_author() {
        let query = SearchQuery::new().author("Williams K");
        assert_eq!(query.build(), "Williams K[Author]");
    }

    #[test]
    fn test_affiliation() {
        let query = SearchQuery::new().affiliation("Harvard Medical School");
        assert_eq!(query.build(), "Harvard Medical School[Affiliation]");
    }

    #[test]
    fn test_orcid() {
        let query = SearchQuery::new().orcid("0000-0001-2345-6789");
        assert_eq!(query.build(), "0000-0001-2345-6789[Author - Identifier]");
    }

    #[test]
    fn test_organism() {
        let query = SearchQuery::new().organism("Mus musculus");
        assert_eq!(query.build(), "Mus musculus[Organism]");
    }

    #[test]
    fn test_organism_with_common_name() {
        let query = SearchQuery::new().organism("mouse");
        assert_eq!(query.build(), "mouse[Organism]");
    }

    #[test]
    fn test_human_studies_only() {
        let query = SearchQuery::new().human_studies_only();
        assert_eq!(query.build(), "humans[mh]");
    }

    #[test]
    fn test_animal_studies_only() {
        let query = SearchQuery::new().animal_studies_only();
        assert_eq!(query.build(), "animals[mh]");
    }

    #[test]
    fn test_age_group() {
        let query = SearchQuery::new().age_group("Child");
        assert_eq!(query.build(), "Child[mh]");
    }

    #[test]
    fn test_custom_filter() {
        let query = SearchQuery::new().custom_filter("custom[field]");
        assert_eq!(query.build(), "custom[field]");
    }

    #[test]
    fn test_combined_advanced_filters() {
        let query = SearchQuery::new()
            .query("cancer treatment")
            .mesh_term("Neoplasms")
            .author("Smith J")
            .human_studies_only()
            .affiliation("Harvard");

        let expected = "cancer treatment AND Neoplasms[MeSH Terms] AND Smith J[Author] AND humans[mh] AND Harvard[Affiliation]";
        assert_eq!(query.build(), expected);
    }

    #[test]
    fn test_empty_mesh_terms_array() {
        let mesh_terms: &[&str] = &[];
        let query = SearchQuery::new().mesh_terms(mesh_terms);
        assert_eq!(query.build(), "");
    }

    #[test]
    fn test_single_mesh_term_via_array() {
        let mesh_terms = ["Neoplasms"];
        let query = SearchQuery::new().mesh_terms(&mesh_terms);
        assert_eq!(query.build(), "Neoplasms[MeSH Terms]");
    }

    #[test]
    fn test_mesh_term_with_spaces() {
        let query = SearchQuery::new().mesh_term("Diabetes Mellitus, Type 2");
        assert_eq!(query.build(), "Diabetes Mellitus, Type 2[MeSH Terms]");
    }

    #[test]
    fn test_author_with_special_characters() {
        let query = SearchQuery::new().author("O'Connor J");
        assert_eq!(query.build(), "O'Connor J[Author]");
    }

    #[test]
    fn test_affiliation_with_special_characters() {
        let query = SearchQuery::new().affiliation("Johns Hopkins & MIT");
        assert_eq!(query.build(), "Johns Hopkins & MIT[Affiliation]");
    }

    #[test]
    fn test_custom_filter_preservation() {
        let query = SearchQuery::new()
            .custom_filter("first[custom]")
            .custom_filter("second[custom]");
        assert_eq!(query.build(), "first[custom] AND second[custom]");
    }
}
