//! Europe PMC subcommands.
//!
//! Europe PMC (<https://europepmc.org>) is a complementary index to the NCBI
//! E-utilities: it covers preprints, patents and agricultural literature in
//! addition to PubMed and PMC, and requires no API key.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use pubmed_client::{EuropePmcId, EuropePmcSearchOptions, EuropePmcSource, ResultType};

use super::{ClientContext, OutputFormat};

/// Level of detail requested from the Europe PMC `search` endpoint.
#[derive(Clone, Debug, ValueEnum)]
pub enum SearchDetail {
    /// Identifiers only
    #[value(name = "idlist", alias = "id-list")]
    IdList,
    /// Core bibliographic fields
    Lite,
    /// Full metadata, including abstracts and citation counts
    Core,
}

impl SearchDetail {
    fn to_result_type(&self) -> ResultType {
        match self {
            SearchDetail::IdList => ResultType::IdList,
            SearchDetail::Lite => ResultType::Lite,
            SearchDetail::Core => ResultType::Core,
        }
    }
}

/// Shared `(source, id)` addressing arguments.
///
/// Europe PMC identifies every record by a source database plus an id. Rather
/// than force both onto the command line, the id is accepted bare
/// (`PMC3258128`, `33515491`), with an explicit `--source`, or fully qualified
/// (`PPR/PPR123456`).
#[derive(Args, Debug)]
pub struct RecordId {
    /// Record id, bare (PMC3258128, 33515491) or qualified (PPR/PPR123456)
    pub id: String,

    /// Source database (MED, PMC, PPR, AGR, CBA, PAT).
    /// Defaults to PMC for PMC-prefixed ids, otherwise MED.
    #[arg(short, long)]
    pub source: Option<String>,
}

impl RecordId {
    fn resolve(&self) -> Result<EuropePmcId> {
        let id = self.id.trim();
        if id.is_empty() {
            bail!("record id must not be empty");
        }

        if id.contains('/') {
            return id
                .parse::<EuropePmcId>()
                .with_context(|| format!("invalid Europe PMC id '{id}'"));
        }

        let source = match self.source.as_deref() {
            Some(source) if !source.trim().is_empty() => EuropePmcSource::from(source),
            _ if id.to_ascii_uppercase().starts_with("PMC") => EuropePmcSource::Pmc,
            _ => EuropePmcSource::Med,
        };

        if source == EuropePmcSource::Pmc {
            return EuropePmcId::pmc(id).with_context(|| format!("invalid PMC id '{id}'"));
        }

        Ok(EuropePmcId::new(source, id))
    }
}

#[derive(Args, Debug)]
pub struct EuropePmc {
    #[command(subcommand)]
    command: EuropePmcCommand,
}

#[derive(Subcommand, Debug)]
enum EuropePmcCommand {
    /// Search Europe PMC across every source it indexes
    Search(Search),
    /// Fetch the full text of a record (parsed sections or raw JATS XML)
    Fulltext(Fulltext),
    /// List the works cited by a record
    References(ListArgs),
    /// List the articles citing a record
    Citations(ListArgs),
    /// List cross-references from a record to external databases
    Links(Links),
    /// Download a record's supplementary-files ZIP archive
    Supplementary(Supplementary),
}

impl EuropePmc {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        match &self.command {
            EuropePmcCommand::Search(cmd) => cmd.execute(ctx).await,
            EuropePmcCommand::Fulltext(cmd) => cmd.execute(ctx).await,
            EuropePmcCommand::References(cmd) => cmd.execute_references(ctx).await,
            EuropePmcCommand::Citations(cmd) => cmd.execute_citations(ctx).await,
            EuropePmcCommand::Links(cmd) => cmd.execute(ctx).await,
            EuropePmcCommand::Supplementary(cmd) => cmd.execute(ctx).await,
        }
    }
}

// ================================================================================================
// search
// ================================================================================================

#[derive(Args, Debug)]
pub struct Search {
    /// Europe PMC query (e.g. "malaria vaccine", "TITLE:CRISPR AND SRC:PPR")
    #[arg(required = true)]
    query: String,

    /// Maximum number of records to return
    #[arg(short, long, default_value = "10")]
    max: usize,

    /// Level of detail to request
    #[arg(long, value_enum, default_value_t = SearchDetail::Lite)]
    result_type: SearchDetail,

    /// Europe PMC sort expression (e.g. "P_PDATE_D desc", "CITED desc")
    #[arg(long)]
    sort: Option<String>,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

impl Search {
    async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.europe_pmc_client();

        tracing::info!(query = %self.query, max = self.max, "Searching Europe PMC");

        let opts = EuropePmcSearchOptions {
            result_type: self.result_type.to_result_type(),
            page_size: self.max.clamp(1, 1000) as u32,
            sort: self.sort.clone(),
            ..Default::default()
        };
        let results = client.search_all(&self.query, self.max, &opts).await?;

        match self.format {
            OutputFormat::Json => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    serde_json::to_string_pretty(&results)?
                )?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(stdout, "Found {} Europe PMC records\n", results.len())?;
                for (i, record) in results.iter().enumerate() {
                    writeln!(
                        stdout,
                        "  {}. {} ({}/{})",
                        i + 1,
                        record.title.as_deref().unwrap_or("Untitled"),
                        record.source,
                        record.id
                    )?;
                    write_field(&mut stdout, "Authors", record.author_string.as_deref())?;
                    write_field(&mut stdout, "Journal", record.journal_title.as_deref())?;
                    write_field(&mut stdout, "Year", record.pub_year.as_deref())?;
                    write_field(&mut stdout, "PMID", record.pmid.as_deref())?;
                    write_field(&mut stdout, "PMC", record.pmcid.as_deref())?;
                    write_field(&mut stdout, "DOI", record.doi.as_deref())?;
                    writeln!(stdout)?;
                }
            }
            _ => bail!(
                "Unsupported format '{}' for europe-pmc search. Use 'text' or 'json'.",
                self.format
            ),
        }

        Ok(())
    }
}

// ================================================================================================
// fulltext
// ================================================================================================

#[derive(Args, Debug)]
pub struct Fulltext {
    #[command(flatten)]
    record: RecordId,

    /// Emit the raw JATS XML instead of parsed sections.
    /// Required for non-PMC sources, which cannot be parsed into an article.
    #[arg(long)]
    xml: bool,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl Fulltext {
    async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let id = self.record.resolve()?;
        let client = ctx.europe_pmc_client();

        tracing::info!(id = %id, xml = self.xml, "Fetching Europe PMC full text");

        let output = if self.xml {
            client.fetch_full_text_xml(&id).await?
        } else {
            let article = client.fetch_full_text(&id).await?;
            let mut text = String::new();
            text.push_str(&format!(
                "Title: {}\n",
                article.title().unwrap_or("Untitled")
            ));
            text.push_str(&format!("PMC ID: {}\n", article.pmcid()));
            if let Some(doi) = article.doi() {
                text.push_str(&format!("DOI: {doi}\n"));
            }
            if !article.authors().is_empty() {
                let authors: Vec<&str> = article
                    .authors()
                    .iter()
                    .map(|a| a.full_name.as_str())
                    .collect();
                text.push_str(&format!("Authors: {}\n", authors.join(", ")));
            }
            if let Some(journal) = article.journal().title.as_deref() {
                text.push_str(&format!("Journal: {journal}\n"));
            }
            for section in article.sections() {
                let title = section
                    .title
                    .as_deref()
                    .or(section.section_type.as_deref())
                    .unwrap_or("Untitled");
                text.push_str(&format!("\n## {title}\n{}\n", section.content));
            }
            text
        };

        match &self.output {
            Some(path) => {
                std::fs::write(path, &output)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                tracing::info!(path = %path.display(), "Wrote Europe PMC full text");
            }
            None => writeln!(std::io::stdout(), "{output}")?,
        }

        Ok(())
    }
}

// ================================================================================================
// references / citations
// ================================================================================================

#[derive(Args, Debug)]
pub struct ListArgs {
    #[command(flatten)]
    record: RecordId,

    /// Maximum number of entries to display
    #[arg(short, long, default_value = "50")]
    max: usize,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

impl ListArgs {
    async fn execute_references(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let id = self.record.resolve()?;
        let client = ctx.europe_pmc_client();

        tracing::info!(id = %id, "Fetching Europe PMC references");

        let references = client.get_references(&id).await?;

        match self.format {
            OutputFormat::Json => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    serde_json::to_string_pretty(&references)?
                )?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(stdout, "{id} cites {} works\n", references.len())?;
                for (i, reference) in references.iter().take(self.max).enumerate() {
                    writeln!(
                        stdout,
                        "  {}. {}",
                        i + 1,
                        reference.title.as_deref().unwrap_or("Untitled")
                    )?;
                    write_field(&mut stdout, "Authors", reference.author_string.as_deref())?;
                    write_field(
                        &mut stdout,
                        "Journal",
                        reference.journal_abbreviation.as_deref(),
                    )?;
                    write_field(&mut stdout, "Year", reference.pub_year.as_deref())?;
                    write_field(&mut stdout, "PMID", reference.pmid.as_deref())?;
                    write_field(&mut stdout, "DOI", reference.doi.as_deref())?;
                }
                write_truncation_note(&mut stdout, references.len(), self.max)?;
            }
            _ => bail!(
                "Unsupported format '{}' for europe-pmc references. Use 'text' or 'json'.",
                self.format
            ),
        }

        Ok(())
    }

    async fn execute_citations(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let id = self.record.resolve()?;
        let client = ctx.europe_pmc_client();

        tracing::info!(id = %id, "Fetching Europe PMC citations");

        let citations = client.get_citations(&id).await?;

        match self.format {
            OutputFormat::Json => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    serde_json::to_string_pretty(&citations)?
                )?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(stdout, "{id} is cited by {} articles\n", citations.len())?;
                for (i, citation) in citations.iter().take(self.max).enumerate() {
                    writeln!(
                        stdout,
                        "  {}. {}",
                        i + 1,
                        citation.title.as_deref().unwrap_or("Untitled")
                    )?;
                    write_field(&mut stdout, "Authors", citation.author_string.as_deref())?;
                    write_field(
                        &mut stdout,
                        "Journal",
                        citation.journal_abbreviation.as_deref(),
                    )?;
                    write_field(&mut stdout, "Year", citation.pub_year.as_deref())?;
                    if let (Some(source), Some(cited_id)) =
                        (citation.source.as_deref(), citation.id.as_deref())
                    {
                        write_field(&mut stdout, "ID", Some(&format!("{source}/{cited_id}")))?;
                    }
                    write_field(&mut stdout, "Cited by", citation.cited_by_count.as_deref())?;
                }
                write_truncation_note(&mut stdout, citations.len(), self.max)?;
            }
            _ => bail!(
                "Unsupported format '{}' for europe-pmc citations. Use 'text' or 'json'.",
                self.format
            ),
        }

        Ok(())
    }
}

// ================================================================================================
// links
// ================================================================================================

#[derive(Args, Debug)]
pub struct Links {
    #[command(flatten)]
    record: RecordId,

    /// Filter to a single external database (e.g. UNIPROT, PDB, EMBL)
    #[arg(long)]
    db: Option<String>,

    /// Maximum number of cross-reference entries to show per database
    #[arg(short, long, default_value = "20")]
    max: usize,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

impl Links {
    async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let id = self.record.resolve()?;
        let client = ctx.europe_pmc_client();

        tracing::info!(id = %id, db = ?self.db, "Fetching Europe PMC database links");

        let links = client.get_database_links(&id).await?;
        let filter = self.db.as_deref().map(str::to_ascii_uppercase);
        let links: Vec<_> = links
            .into_iter()
            .filter(|link| match (&filter, link.db_name.as_deref()) {
                (Some(filter), Some(name)) => name.to_ascii_uppercase() == *filter,
                (Some(_), None) => false,
                (None, _) => true,
            })
            .collect();

        match self.format {
            OutputFormat::Json => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    serde_json::to_string_pretty(&links)?
                )?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                if links.is_empty() {
                    writeln!(stdout, "{id} has no external database cross-references")?;
                    return Ok(());
                }
                writeln!(stdout, "{id} links to {} database(s)\n", links.len())?;
                for link in &links {
                    let count = link.db_count.unwrap_or(link.info.len() as u32);
                    writeln!(
                        stdout,
                        "  {} ({count} cross-reference(s))",
                        link.db_name.as_deref().unwrap_or("Unknown database")
                    )?;
                    for entry in link.info.iter().take(self.max) {
                        // Europe PMC documents the four info slots only
                        // positionally, so show whichever are populated.
                        let values: Vec<&str> = [
                            entry.info1.as_deref(),
                            entry.info2.as_deref(),
                            entry.info3.as_deref(),
                            entry.info4.as_deref(),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|v| !v.trim().is_empty())
                        .collect();
                        if !values.is_empty() {
                            writeln!(stdout, "    - {}", values.join(" | "))?;
                        }
                    }
                    if link.info.len() > self.max {
                        writeln!(
                            stdout,
                            "    ... and {} more (use --max to show more)",
                            link.info.len() - self.max
                        )?;
                    }
                    writeln!(stdout)?;
                }
            }
            _ => bail!(
                "Unsupported format '{}' for europe-pmc links. Use 'text' or 'json'.",
                self.format
            ),
        }

        Ok(())
    }
}

// ================================================================================================
// supplementary
// ================================================================================================

#[derive(Args, Debug)]
pub struct Supplementary {
    #[command(flatten)]
    record: RecordId,

    /// Path of the ZIP file to write (default: <ID>_supplementary.zip)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl Supplementary {
    async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let id = self.record.resolve()?;
        let client = ctx.europe_pmc_client();

        let output = self
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("{}_supplementary.zip", id.id)));

        tracing::info!(id = %id, path = %output.display(), "Downloading supplementary files");

        let written = client.download_supplementary_files(&id, &output).await?;
        writeln!(std::io::stdout(), "Wrote {}", written.display())?;

        Ok(())
    }
}

// ================================================================================================
// Shared formatting helpers
// ================================================================================================

/// Write a labelled line, skipping values that are absent or blank.
fn write_field(out: &mut impl Write, label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) {
        writeln!(out, "     {label}: {value}")?;
    }
    Ok(())
}

/// Note how many entries were withheld by `--max`, if any.
fn write_truncation_note(out: &mut impl Write, total: usize, max: usize) -> Result<()> {
    if total > max {
        writeln!(
            out,
            "\n  ... and {} more (use --max to show more)",
            total - max
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, source: Option<&str>) -> RecordId {
        RecordId {
            id: id.to_string(),
            source: source.map(str::to_string),
        }
    }

    #[test]
    fn bare_pmc_id_defaults_to_pmc_source() {
        assert_eq!(
            record("PMC3258128", None).resolve().unwrap().to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn bare_numeric_id_defaults_to_med_source() {
        assert_eq!(
            record("33515491", None).resolve().unwrap().to_string(),
            "MED/33515491"
        );
    }

    #[test]
    fn explicit_pmc_source_normalizes_a_bare_number() {
        assert_eq!(
            record("3258128", Some("pmc"))
                .resolve()
                .unwrap()
                .to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn qualified_id_wins_over_source_flag() {
        let id = record("PPR/PPR123456", Some("MED")).resolve().unwrap();
        assert_eq!(id.source, EuropePmcSource::Ppr);
        assert_eq!(id.id, "PPR123456");
    }

    #[test]
    fn unknown_source_is_passed_through() {
        assert_eq!(
            record("42", Some("xyz")).resolve().unwrap().to_string(),
            "XYZ/42"
        );
    }

    #[test]
    fn invalid_ids_are_rejected() {
        assert!(record("  ", None).resolve().is_err());
        assert!(record("MED/", None).resolve().is_err());
        assert!(record("not-a-pmcid", Some("PMC")).resolve().is_err());
    }
}
