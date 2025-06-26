//! Date types and utilities for PubMed query date filtering

/// Represents a date for PubMed searches with varying precision
#[derive(Debug, Clone, PartialEq)]
pub struct PubDate {
    year: u32,
    month: Option<u32>,
    day: Option<u32>,
}

impl PubDate {
    /// Create a new PubDate with year only
    pub fn new(year: u32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    /// Create a new PubDate with year and month
    pub fn with_month(year: u32, month: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: None,
        }
    }

    /// Create a new PubDate with year, month, and day
    pub fn with_day(year: u32, month: u32, day: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: Some(day),
        }
    }

    /// Format as PubMed date string
    pub fn to_pubmed_string(&self) -> String {
        match (self.month, self.day) {
            (Some(month), Some(day)) => format!("{}/{:02}/{:02}", self.year, month, day),
            (Some(month), None) => format!("{}/{:02}", self.year, month),
            _ => self.year.to_string(),
        }
    }
}

impl From<u32> for PubDate {
    fn from(year: u32) -> Self {
        Self::new(year)
    }
}

impl From<(u32, u32)> for PubDate {
    fn from((year, month): (u32, u32)) -> Self {
        Self::with_month(year, month)
    }
}

impl From<(u32, u32, u32)> for PubDate {
    fn from((year, month, day): (u32, u32, u32)) -> Self {
        Self::with_day(year, month, day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubdate_new() {
        let date = PubDate::new(2023);
        assert_eq!(date.to_pubmed_string(), "2023");
    }

    #[test]
    fn test_pubdate_with_month() {
        let date = PubDate::with_month(2023, 6);
        assert_eq!(date.to_pubmed_string(), "2023/06");
    }

    #[test]
    fn test_pubdate_with_day() {
        let date = PubDate::with_day(2023, 6, 15);
        assert_eq!(date.to_pubmed_string(), "2023/06/15");
    }

    #[test]
    fn test_pubdate_from_u32() {
        let date: PubDate = 2023.into();
        assert_eq!(date.to_pubmed_string(), "2023");
    }

    #[test]
    fn test_pubdate_from_tuple_month() {
        let date: PubDate = (2023, 6).into();
        assert_eq!(date.to_pubmed_string(), "2023/06");
    }

    #[test]
    fn test_pubdate_from_tuple_day() {
        let date: PubDate = (2023, 6, 15).into();
        assert_eq!(date.to_pubmed_string(), "2023/06/15");
    }

    #[test]
    fn test_pubdate_equality() {
        let date1 = PubDate::new(2023);
        let date2 = PubDate::new(2023);
        let date3 = PubDate::new(2024);

        assert_eq!(date1, date2);
        assert_ne!(date1, date3);

        let date_month1 = PubDate::with_month(2023, 6);
        let date_month2 = PubDate::with_month(2023, 6);
        let date_month3 = PubDate::with_month(2023, 7);

        assert_eq!(date_month1, date_month2);
        assert_ne!(date_month1, date_month3);
        assert_ne!(date1, date_month1); // Different precision
    }

    #[test]
    fn test_pubdate_clone() {
        let original = PubDate::with_day(2023, 12, 25);
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.to_pubmed_string(), cloned.to_pubmed_string());
    }

    #[test]
    fn test_pubdate_debug_format() {
        let date = PubDate::with_month(2023, 6);
        let debug_str = format!("{:?}", date);
        assert!(debug_str.contains("2023"));
        assert!(debug_str.contains("6"));
    }

    #[test]
    fn test_month_padding() {
        let date = PubDate::with_month(2023, 1);
        assert_eq!(date.to_pubmed_string(), "2023/01");

        let date = PubDate::with_month(2023, 12);
        assert_eq!(date.to_pubmed_string(), "2023/12");
    }

    #[test]
    fn test_day_padding() {
        let date = PubDate::with_day(2023, 1, 5);
        assert_eq!(date.to_pubmed_string(), "2023/01/05");

        let date = PubDate::with_day(2023, 12, 25);
        assert_eq!(date.to_pubmed_string(), "2023/12/25");
    }

    #[test]
    fn test_edge_case_dates() {
        // Test minimum values
        let min_date = PubDate::with_day(1, 1, 1);
        assert_eq!(min_date.to_pubmed_string(), "1/01/01");

        // Test typical boundary dates
        let leap_year = PubDate::with_day(2024, 2, 29);
        assert_eq!(leap_year.to_pubmed_string(), "2024/02/29");

        // Test far future date (used internally)
        let future_date = PubDate::new(3000);
        assert_eq!(future_date.to_pubmed_string(), "3000");
    }

    #[test]
    fn test_date_precision_consistency() {
        // Year only should not have month/day
        let year_only = PubDate::new(2023);
        assert_eq!(year_only.year, 2023);
        assert_eq!(year_only.month, None);
        assert_eq!(year_only.day, None);

        // Month precision should not have day
        let month_precision = PubDate::with_month(2023, 6);
        assert_eq!(month_precision.year, 2023);
        assert_eq!(month_precision.month, Some(6));
        assert_eq!(month_precision.day, None);

        // Day precision should have all fields
        let day_precision = PubDate::with_day(2023, 6, 15);
        assert_eq!(day_precision.year, 2023);
        assert_eq!(day_precision.month, Some(6));
        assert_eq!(day_precision.day, Some(15));
    }
}
