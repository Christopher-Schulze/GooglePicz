use chrono::{DateTime, Utc};

pub fn parse_date_query(query: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    use chrono::{NaiveDate, TimeZone};
    if let Some(idx) = query.find("..") {
        let start_str = &query[..idx];
        let end_str = &query[idx + 2..];
        if let (Ok(s), Ok(e)) = (
            NaiveDate::parse_from_str(start_str, "%Y-%m-%d"),
            NaiveDate::parse_from_str(end_str, "%Y-%m-%d"),
        ) {
            let start = Utc.from_utc_datetime(&s.and_hms_opt(0, 0, 0)?);
            let end = Utc.from_utc_datetime(&e.and_hms_opt(23, 59, 59)?);
            return Some((start, end));
        }
    } else if let Ok(d) = NaiveDate::parse_from_str(query, "%Y-%m-%d") {
        let start = Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?);
        let end = Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59)?);
        return Some((start, end));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Filename,
    Favoriten,
    DateRange,
}

impl SearchMode {
    pub const ALL: [SearchMode; 3] = [SearchMode::Filename, SearchMode::Favoriten, SearchMode::DateRange];
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SearchMode::Filename => "Filename",
            SearchMode::Favoriten => "Favoriten",
            SearchMode::DateRange => "Datum von/bis",
        };
        write!(f, "{}", s)
    }
}
