//! Date filtering methods for PubMed search queries

use super::{PubDate, SearchQuery};

impl SearchQuery {
    /// Filter by publication date range
    ///
    /// # Arguments
    ///
    /// * `start_year` - Start year (inclusive)
    /// * `end_year` - End year (inclusive, optional)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("immunotherapy")
    ///     .date_range(2020, Some(2023));
    /// ```
    pub fn date_range(mut self, start_year: u32, end_year: Option<u32>) -> Self {
        let date_filter = match end_year {
            Some(end) => format!("{start_year}:{end}[pdat]"),
            None => format!("{start_year}:3000[pdat]"), // Far future date
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter by publication date range with flexible precision
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // Year range
    /// let query = SearchQuery::new()
    ///     .query("covid vaccines")
    ///     .published_between(2020, Some(2023));
    ///
    /// // Month precision
    /// let query = SearchQuery::new()
    ///     .query("pandemic response")
    ///     .published_between((2020, 3), Some((2021, 12)));
    ///
    /// // Day precision
    /// let query = SearchQuery::new()
    ///     .query("outbreak analysis")
    ///     .published_between((2020, 3, 15), Some((2020, 12, 31)));
    ///
    /// // Open-ended (from date onwards)
    /// let query = SearchQuery::new()
    ///     .query("recent research")
    ///     .published_between(2023, None::<u32>);
    /// ```
    pub fn published_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[pdat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[pdat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter to articles published in a specific year
    ///
    /// # Arguments
    ///
    /// * `year` - Year to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("artificial intelligence")
    ///     .published_in_year(2023);
    /// ```
    pub fn published_in_year(mut self, year: u32) -> Self {
        self.filters.push(format!("{year}[pdat]"));
        self
    }

    /// Filter by entry date (when added to PubMed database)
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("recent discoveries")
    ///     .entry_date_between(2023, Some(2024));
    /// ```
    pub fn entry_date_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[edat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[edat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter by modification date (when last updated in PubMed database)
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("updated articles")
    ///     .modification_date_between(2023, None::<u32>);
    /// ```
    pub fn modification_date_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[mdat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[mdat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter to articles published after a specific date
    ///
    /// # Arguments
    ///
    /// * `date` - Date after which articles were published (can be year, (year, month), or (year, month, day))
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // After a specific year
    /// let query = SearchQuery::new()
    ///     .query("crispr")
    ///     .published_after(2020);
    ///
    /// // After a specific month
    /// let query = SearchQuery::new()
    ///     .query("covid treatment")
    ///     .published_after((2020, 3));
    ///
    /// // After a specific date
    /// let query = SearchQuery::new()
    ///     .query("pandemic response")
    ///     .published_after((2020, 3, 15));
    /// ```
    pub fn published_after<D>(self, date: D) -> Self
    where
        D: Into<PubDate>,
    {
        self.published_between(date, None::<u32>)
    }

    /// Filter to articles published before a specific date
    ///
    /// # Arguments
    ///
    /// * `date` - Date before which articles were published (can be year, (year, month), or (year, month, day))
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // Before a specific year
    /// let query = SearchQuery::new()
    ///     .query("genome sequencing")
    ///     .published_before(2020);
    ///
    /// // Before a specific month
    /// let query = SearchQuery::new()
    ///     .query("early research")
    ///     .published_before((2020, 3));
    ///
    /// // Before a specific date
    /// let query = SearchQuery::new()
    ///     .query("pre-pandemic studies")
    ///     .published_before((2020, 3, 15));
    /// ```
    pub fn published_before<D>(self, date: D) -> Self
    where
        D: Into<PubDate>,
    {
        self.published_between(1900, Some(date))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_range_with_end_year() {
        let query = SearchQuery::new().date_range(2020, Some(2023));
        assert_eq!(query.build(), "2020:2023[pdat]");
    }

    #[test]
    fn test_date_range_without_end_year() {
        let query = SearchQuery::new().date_range(2020, None);
        assert_eq!(query.build(), "2020:3000[pdat]");
    }

    #[test]
    fn test_published_in_year() {
        let query = SearchQuery::new().published_in_year(2023);
        assert_eq!(query.build(), "2023[pdat]");
    }

    #[test]
    fn test_published_between_years() {
        let query = SearchQuery::new().published_between(2020, Some(2023));
        assert_eq!(query.build(), "2020:2023[pdat]");
    }

    #[test]
    fn test_published_between_months() {
        let query = SearchQuery::new().published_between((2020, 3), Some((2021, 12)));
        assert_eq!(query.build(), "2020/03:2021/12[pdat]");
    }

    #[test]
    fn test_published_between_days() {
        let query = SearchQuery::new().published_between((2020, 3, 15), Some((2020, 12, 31)));
        assert_eq!(query.build(), "2020/03/15:2020/12/31[pdat]");
    }

    #[test]
    fn test_published_after_year() {
        let query = SearchQuery::new().published_after(2020);
        assert_eq!(query.build(), "2020:3000[pdat]");
    }

    #[test]
    fn test_published_after_month() {
        let query = SearchQuery::new().published_after((2020, 3));
        assert_eq!(query.build(), "2020/03:3000[pdat]");
    }

    #[test]
    fn test_published_after_day() {
        let query = SearchQuery::new().published_after((2020, 3, 15));
        assert_eq!(query.build(), "2020/03/15:3000[pdat]");
    }

    #[test]
    fn test_published_before_year() {
        let query = SearchQuery::new().published_before(2020);
        assert_eq!(query.build(), "1900:2020[pdat]");
    }

    #[test]
    fn test_published_before_month() {
        let query = SearchQuery::new().published_before((2020, 3));
        assert_eq!(query.build(), "1900:2020/03[pdat]");
    }

    #[test]
    fn test_published_before_day() {
        let query = SearchQuery::new().published_before((2020, 3, 15));
        assert_eq!(query.build(), "1900:2020/03/15[pdat]");
    }

    #[test]
    fn test_entry_date_between() {
        let query = SearchQuery::new().entry_date_between(2023, Some(2024));
        assert_eq!(query.build(), "2023:2024[edat]");
    }

    #[test]
    fn test_entry_date_between_open_ended() {
        let query = SearchQuery::new().entry_date_between((2023, 6), None::<u32>);
        assert_eq!(query.build(), "2023/06:3000[edat]");
    }

    #[test]
    fn test_modification_date_between() {
        let query =
            SearchQuery::new().modification_date_between((2023, 1, 1), Some((2023, 12, 31)));
        assert_eq!(query.build(), "2023/01/01:2023/12/31[mdat]");
    }

    #[test]
    fn test_modification_date_between_open_ended() {
        let query = SearchQuery::new().modification_date_between(2023, None::<u32>);
        assert_eq!(query.build(), "2023:3000[mdat]");
    }

    #[test]
    fn test_combined_date_filters() {
        let query = SearchQuery::new()
            .query("covid-19")
            .published_between(2020, Some(2023))
            .entry_date_between((2023, 1), None::<u32>);

        assert_eq!(
            query.build(),
            "covid-19 AND 2020:2023[pdat] AND 2023/01:3000[edat]"
        );
    }

    #[test]
    fn test_multiple_date_ranges() {
        let query = SearchQuery::new()
            .date_range(2020, Some(2021))
            .date_range(2022, Some(2023));

        assert_eq!(query.build(), "2020:2021[pdat] AND 2022:2023[pdat]");
    }

    #[test]
    fn test_date_precision_mixing() {
        let query = SearchQuery::new()
            .published_between(2020, Some((2023, 6, 15)))
            .entry_date_between((2023, 1), Some(2024));

        let built = query.build();
        assert!(built.contains("2020:2023/06/15[pdat]"));
        assert!(built.contains("2023/01:2024[edat]"));
    }

    #[test]
    fn test_multiple_date_types() {
        let query = SearchQuery::new()
            .query("research")
            .published_between(2020, Some(2023))
            .entry_date_between((2023, 1), Some((2023, 12)))
            .modification_date_between((2023, 6), None::<u32>);

        let built = query.build();
        assert!(built.contains("2020:2023[pdat]"));
        assert!(built.contains("2023/01:2023/12[edat]"));
        assert!(built.contains("2023/06:3000[mdat]"));
    }

    #[test]
    fn test_edge_year_values() {
        let query = SearchQuery::new().date_range(1, Some(9999));
        assert_eq!(query.build(), "1:9999[pdat]");
    }

    #[test]
    fn test_same_start_end_date() {
        let query = SearchQuery::new().published_between(2023, Some(2023));
        assert_eq!(query.build(), "2023:2023[pdat]");
    }

    #[test]
    fn test_published_between_none_end() {
        let query = SearchQuery::new().published_between(2020, None::<u32>);
        assert_eq!(query.build(), "2020:3000[pdat]");
    }
}
