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
