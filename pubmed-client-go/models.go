package pubmedclient

import (
	"encoding/json"
	"fmt"
	"strings"
)

// The structs below mirror the JSON produced by the Rust side. They
// deliberately cover the commonly used fields rather than every field of the
// JATS domain model: unknown keys are ignored by encoding/json, so the Rust
// types can grow without breaking this package.

// Affiliation is an author's institutional affiliation.
type Affiliation struct {
	ID          string `json:"id,omitempty"`
	Institution string `json:"institution,omitempty"`
	Department  string `json:"department,omitempty"`
	Address     string `json:"address,omitempty"`
	Country     string `json:"country,omitempty"`
}

// Author is a contributor to an article or reference.
type Author struct {
	Surname    string `json:"surname,omitempty"`
	GivenNames string `json:"given_names,omitempty"`
	Initials   string `json:"initials,omitempty"`
	Suffix     string `json:"suffix,omitempty"`
	// FullName is the formatted name, always populated.
	FullName     string        `json:"full_name"`
	Affiliations []Affiliation `json:"affiliations,omitempty"`
	ORCID        string        `json:"orcid,omitempty"`
	Email        string        `json:"email,omitempty"`
	// IsCorresponding reports whether this author is a corresponding author.
	IsCorresponding bool     `json:"is_corresponding"`
	Roles           []string `json:"roles,omitempty"`
	// CollabName is set when the contributor is a collective (a group author)
	// rather than an individual; SurName and GivenNames are then empty.
	CollabName string `json:"collab_name,omitempty"`
}

// AbstractSection is one labeled section of a structured abstract, such as
// "BACKGROUND" or "METHODS".
type AbstractSection struct {
	Label string `json:"label"`
	Text  string `json:"text"`
}

// MeshQualifier is a subheading attached to a MeSH descriptor.
type MeshQualifier struct {
	QualifierName string `json:"qualifier_name"`
	QualifierUI   string `json:"qualifier_ui"`
	MajorTopic    bool   `json:"major_topic"`
}

// MeshTerm is a MeSH descriptor assigned to an article.
type MeshTerm struct {
	DescriptorName string          `json:"descriptor_name"`
	DescriptorUI   string          `json:"descriptor_ui"`
	MajorTopic     bool            `json:"major_topic"`
	Qualifiers     []MeshQualifier `json:"qualifiers,omitempty"`
}

// MeshHeading groups the MeSH terms and supplemental concepts of an article.
type MeshHeading struct {
	MeshTerms []MeshTerm `json:"mesh_terms,omitempty"`
}

// ChemicalConcept is a chemical substance indexed for an article.
type ChemicalConcept struct {
	Name           string `json:"name"`
	RegistryNumber string `json:"registry_number,omitempty"`
	UI             string `json:"ui,omitempty"`
}

// Article is PubMed metadata for a single article, as returned by
// [Client.FetchArticle], [Client.FetchArticles] and [Client.SearchAndFetch].
type Article struct {
	PMID        string   `json:"pmid"`
	Title       string   `json:"title"`
	Authors     []Author `json:"authors,omitempty"`
	AuthorCount uint32   `json:"author_count"`
	Journal     string   `json:"journal"`
	PubDate     string   `json:"pub_date"`
	DOI         string   `json:"doi,omitempty"`
	// PMCID is the PMC identifier (with the "PMC" prefix) when the article has
	// PMC full text, and empty otherwise.
	PMCID              string            `json:"pmc_id,omitempty"`
	AbstractText       string            `json:"abstract_text,omitempty"`
	StructuredAbstract []AbstractSection `json:"structured_abstract,omitempty"`
	ArticleTypes       []string          `json:"article_types,omitempty"`
	MeshHeadings       []MeshHeading     `json:"mesh_headings,omitempty"`
	Keywords           []string          `json:"keywords,omitempty"`
	ChemicalList       []ChemicalConcept `json:"chemical_list,omitempty"`
	Volume             string            `json:"volume,omitempty"`
	Issue              string            `json:"issue,omitempty"`
	Pages              string            `json:"pages,omitempty"`
	Language           string            `json:"language,omitempty"`
	JournalAbbrev      string            `json:"journal_abbreviation,omitempty"`
	ISSN               string            `json:"issn,omitempty"`
}

// Journal is the journal metadata of a PMC article.
type Journal struct {
	Title          string `json:"title,omitempty"`
	Abbreviation   string `json:"abbreviation,omitempty"`
	ISSNPrint      string `json:"issn_print,omitempty"`
	ISSNElectronic string `json:"issn_electronic,omitempty"`
	Publisher      string `json:"publisher,omitempty"`
}

// Figure is a figure within a PMC article body.
type Figure struct {
	ID          string `json:"id"`
	Label       string `json:"label,omitempty"`
	Caption     string `json:"caption,omitempty"`
	AltText     string `json:"alt_text,omitempty"`
	FigType     string `json:"fig_type,omitempty"`
	GraphicHref string `json:"graphic_href,omitempty"`
}

// Table is a table within a PMC article body. Only the identifying fields are
// mirrored here; use [Client.FetchMarkdown] when the rendered content is
// needed.
type Table struct {
	ID      string `json:"id"`
	Label   string `json:"label,omitempty"`
	Caption string `json:"caption,omitempty"`
}

// Section is a section of a PMC article body. Sections nest: Subsections holds
// the child sections in document order.
type Section struct {
	ID string `json:"id,omitempty"`
	// SectionType is the JATS sec-type, e.g. "intro", "methods", "results".
	SectionType string `json:"section_type,omitempty"`
	Label       string `json:"label,omitempty"`
	Title       string `json:"title,omitempty"`
	// Content is the concatenated paragraph text of this section, excluding
	// subsections.
	Content     string    `json:"content"`
	Subsections []Section `json:"subsections,omitempty"`
	Figures     []Figure  `json:"figures,omitempty"`
	Tables      []Table   `json:"tables,omitempty"`
	// CitedReferenceIDs are the reference ids cited in this section, in
	// document order. Each matches a [Reference.ID] in [PMCArticle.References].
	CitedReferenceIDs []string `json:"cited_reference_ids,omitempty"`
}

// Reference is a bibliographic reference from a PMC article's reference list.
type Reference struct {
	ID              string   `json:"id"`
	PublicationType string   `json:"publication_type,omitempty"`
	Title           string   `json:"title,omitempty"`
	Authors         []Author `json:"authors,omitempty"`
	Source          string   `json:"source,omitempty"`
	Year            string   `json:"year,omitempty"`
	Volume          string   `json:"volume,omitempty"`
	Issue           string   `json:"issue,omitempty"`
	Pages           string   `json:"pages,omitempty"`
	ElocationID     string   `json:"elocation_id,omitempty"`
	PublisherName   string   `json:"publisher_name,omitempty"`
	PublisherLoc    string   `json:"publisher_loc,omitempty"`
	DOI             string   `json:"doi,omitempty"`
	PMID            string   `json:"pmid,omitempty"`
}

// PMCArticle is the full text of a PMC article, as returned by
// [Client.FetchFullText].
//
// It is a flattened projection of the JATS front/body/back tree: the fields
// below come from the Rust article's accessor methods, so callers do not have
// to walk the DTD structure.
type PMCArticle struct {
	// PMCID is the PMC identifier, e.g. "PMC7906746".
	PMCID string `json:"pmcid"`
	// PMID is the PubMed identifier when the article carries one.
	PMID         string   `json:"pmid,omitempty"`
	Title        string   `json:"title,omitempty"`
	DOI          string   `json:"doi,omitempty"`
	Journal      Journal  `json:"journal"`
	Volume       string   `json:"volume,omitempty"`
	Issue        string   `json:"issue,omitempty"`
	AbstractText string   `json:"abstract_text,omitempty"`
	Keywords     []string `json:"keywords,omitempty"`
	Authors      []Author `json:"authors,omitempty"`
	// Sections are the top-level body sections, each of which may nest further.
	Sections   []Section   `json:"sections,omitempty"`
	References []Reference `json:"references,omitempty"`
	// Figures are all figures across the whole body, flattened out of the
	// section tree for convenience.
	Figures []Figure `json:"figures,omitempty"`
	// FigureCount and TableCount count every figure/table in the body,
	// including those inside nested subsections.
	FigureCount int `json:"figure_count"`
	TableCount  int `json:"table_count"`
}

// SearchFullTextResult pairs a PubMed record with its PMC full text, as
// returned by [Client.SearchWithFullText].
type SearchFullTextResult struct {
	Article Article `json:"article"`
	// FullText is nil when the article is not in the PMC Open Access subset,
	// which is the case for most PubMed articles.
	FullText *PMCArticle `json:"full_text"`
}

// ArticleSummary is the lightweight ESummary record for an article, as returned
// by [Client.FetchSummaries].
//
// Compared with [Article] it omits the abstract, MeSH headings and chemical
// list, which makes it much cheaper for large result sets.
type ArticleSummary struct {
	PMID string `json:"pmid"`
	// Authors are formatted names ("Zhu N"), not the structured [Author] the
	// EFetch API returns.
	Authors []string `json:"authors,omitempty"`
	Title   string   `json:"title"`
	// Journal is the abbreviated journal name; FullJournalName is spelled out.
	Journal         string `json:"journal"`
	FullJournalName string `json:"full_journal_name"`
	PubDate         string `json:"pub_date"`
	// EpubDate is the electronic publication date, which can precede PubDate.
	EpubDate string `json:"epub_date"`
	DOI      string `json:"doi,omitempty"`
	PMCID    string `json:"pmc_id,omitempty"`
	Volume   string `json:"volume"`
	Issue    string `json:"issue"`
	Pages    string `json:"pages"`
	// Languages holds ISO 639-2 codes, e.g. ["eng"].
	Languages []string `json:"languages,omitempty"`
	// PubTypes holds publication types, e.g. ["Journal Article", "Review"].
	PubTypes []string `json:"pub_types,omitempty"`
	ISSN     string   `json:"issn"`
	// ESSN is the electronic ISSN.
	ESSN string `json:"essn"`
	// SortPubDate is the normalized date PubMed sorts on, e.g.
	// "2020/02/20 00:00".
	SortPubDate string `json:"sort_pub_date"`
	// PMCRefCount is how many PMC articles cite this one.
	PMCRefCount uint64 `json:"pmc_ref_count"`
	// RecordStatus is NCBI's status for the record, e.g. "PubMed - indexed for
	// MEDLINE".
	RecordStatus string `json:"record_status,omitempty"`
}

// RelatedArticles is the result of [Client.GetRelatedArticles].
type RelatedArticles struct {
	// SourcePMIDs are the PMIDs that were queried.
	SourcePMIDs []uint32 `json:"source_pmids"`
	// RelatedPMIDs are the PMIDs PubMed considers related, across all sources
	// combined.
	RelatedPMIDs []uint32 `json:"related_pmids"`
	// LinkType is the ELink relationship used, e.g. "pubmed_pubmed".
	LinkType string `json:"link_type"`
}

// PMCLinks is the result of [Client.GetPMCLinks].
type PMCLinks struct {
	SourcePMIDs []uint32 `json:"source_pmids"`
	// PMCIDs are the PMC identifiers with full text available. There is no
	// positional correspondence with SourcePMIDs: articles without PMC full
	// text simply contribute nothing.
	PMCIDs []string `json:"pmc_ids"`
}

// Citations is the result of [Client.GetCitations].
type Citations struct {
	SourcePMIDs []uint32 `json:"source_pmids"`
	// CitingPMIDs are the articles citing the sources, as far as PMC's
	// citation index covers them.
	CitingPMIDs []uint32 `json:"citing_pmids"`
	LinkType    string   `json:"link_type"`
}

// FieldInfo describes one searchable field of an NCBI database.
type FieldInfo struct {
	// Name is the short tag used in queries, e.g. "TITL".
	Name string `json:"name"`
	// FullName is the human-readable name, e.g. "Title".
	FullName    string `json:"full_name"`
	Description string `json:"description"`
	// TermCount is how many distinct terms are indexed, when NCBI reports it.
	TermCount   *uint64 `json:"term_count"`
	IsDate      bool    `json:"is_date"`
	IsNumerical bool    `json:"is_numerical"`
	SingleToken bool    `json:"single_token"`
	Hierarchy   bool    `json:"hierarchy"`
	IsHidden    bool    `json:"is_hidden"`
}

// LinkInfo describes one link from an NCBI database to another.
type LinkInfo struct {
	Name        string `json:"name"`
	Menu        string `json:"menu"`
	Description string `json:"description"`
	// TargetDB is the database the link points at, e.g. "pmc".
	TargetDB string `json:"target_db"`
}

// DatabaseInfo describes one NCBI database, as returned by
// [Client.GetDatabaseInfo].
type DatabaseInfo struct {
	// Name is the internal database name, e.g. "pubmed".
	Name string `json:"name"`
	// MenuName is the display name, e.g. "PubMed".
	MenuName    string `json:"menu_name"`
	Description string `json:"description"`
	Build       string `json:"build,omitempty"`
	// Count is the number of records, when NCBI reports it.
	Count      *uint64 `json:"count"`
	LastUpdate string  `json:"last_update,omitempty"`
	// Fields are the searchable fields; Links are the databases reachable
	// through ELink.
	Fields []FieldInfo `json:"fields,omitempty"`
	Links  []LinkInfo  `json:"links,omitempty"`
}

// SpelledQuerySegment is one piece of a spell-checked query: either a term
// PubMed left alone or the replacement it suggests.
type SpelledQuerySegment struct {
	// Text is the segment as it appears in the corrected query.
	Text string
	// Replaced reports whether this segment is a correction rather than an
	// unchanged part of the original query.
	Replaced bool
}

// UnmarshalJSON decodes the externally tagged enum the Rust side emits —
// {"Original": "..."} or {"Replaced": "..."} — into a flat struct.
func (s *SpelledQuerySegment) UnmarshalJSON(data []byte) error {
	var tagged map[string]string
	if err := json.Unmarshal(data, &tagged); err != nil {
		return err
	}
	if text, ok := tagged["Replaced"]; ok {
		s.Text, s.Replaced = text, true
		return nil
	}
	if text, ok := tagged["Original"]; ok {
		s.Text, s.Replaced = text, false
		return nil
	}
	return fmt.Errorf("unrecognized spelled query segment: %s", data)
}

// SpellCheckResult is the result of [Client.SpellCheck].
type SpellCheckResult struct {
	// Database is the database that was queried, e.g. "pubmed".
	Database string `json:"database"`
	// Query is the term as submitted.
	Query string `json:"query"`
	// CorrectedQuery is PubMed's suggestion, empty when it has none.
	CorrectedQuery string `json:"corrected_query"`
	// SpelledQuery breaks the suggestion into changed and unchanged segments.
	SpelledQuery []SpelledQuerySegment `json:"spelled_query,omitempty"`
}

// HasCorrections reports whether PubMed suggested anything. An empty
// CorrectedQuery, or one identical to the original, means the term was already
// spelled as PubMed expects.
func (s *SpellCheckResult) HasCorrections() bool {
	return s.CorrectedQuery != "" && s.CorrectedQuery != s.Query
}

// Replacements returns only the corrected segments.
func (s *SpellCheckResult) Replacements() []string {
	var replacements []string
	for _, segment := range s.SpelledQuery {
		if segment.Replaced {
			replacements = append(replacements, segment.Text)
		}
	}
	return replacements
}

// DatabaseCount is the number of records matching a term in one NCBI database.
type DatabaseCount struct {
	// DBName is the internal name, e.g. "pubmed"; MenuName is the display name.
	DBName   string `json:"db_name"`
	MenuName string `json:"menu_name"`
	Count    uint64 `json:"count"`
	// Status is NCBI's per-database status, "Ok" when the count is meaningful.
	Status string `json:"status"`
}

// GlobalQueryResults is the result of [Client.GlobalQuery].
type GlobalQueryResults struct {
	// Term is the query that was counted.
	Term string `json:"term"`
	// Results holds one entry per Entrez database, including those with no
	// matches.
	Results []DatabaseCount `json:"results,omitempty"`
}

// NonZero returns only the databases with at least one match.
func (g *GlobalQueryResults) NonZero() []DatabaseCount {
	var matched []DatabaseCount
	for _, result := range g.Results {
		if result.Count > 0 {
			matched = append(matched, result)
		}
	}
	return matched
}

// CountFor returns the match count for one database, and whether it appeared in
// the results at all.
func (g *GlobalQueryResults) CountFor(dbName string) (uint64, bool) {
	for _, result := range g.Results {
		if result.DBName == dbName {
			return result.Count, true
		}
	}
	return 0, false
}

// CitationQuery is one citation to resolve through [Client.MatchCitations].
//
// NCBI matches on the combination, so partial citations often still resolve.
// Values are matched case-insensitively.
type CitationQuery struct {
	// Journal is the journal title abbreviation, e.g.
	// "proc natl acad sci u s a".
	Journal string `json:"journal"`
	Year    string `json:"year"`
	Volume  string `json:"volume"`
	// FirstPage is the first page number, e.g. "3248".
	FirstPage string `json:"first_page"`
	// AuthorName is the first author, e.g. "mann bj".
	AuthorName string `json:"author_name"`
	// Key is a caller-chosen identifier echoed back on the matching
	// [CitationMatch], so results can be paired regardless of order.
	Key string `json:"key"`
}

// CitationMatchStatus reports how a citation resolved.
type CitationMatchStatus string

const (
	// CitationFound means exactly one PMID matched.
	CitationFound CitationMatchStatus = "Found"
	// CitationNotFound means nothing matched.
	CitationNotFound CitationMatchStatus = "NotFound"
	// CitationAmbiguous means several PMIDs matched, so none is returned.
	CitationAmbiguous CitationMatchStatus = "Ambiguous"
)

// CitationMatch is the outcome for one [CitationQuery]. The query fields are
// echoed back so a match stands on its own.
type CitationMatch struct {
	Journal    string `json:"journal"`
	Year       string `json:"year"`
	Volume     string `json:"volume"`
	FirstPage  string `json:"first_page"`
	AuthorName string `json:"author_name"`
	Key        string `json:"key"`
	// PMID is empty unless Status is [CitationFound].
	PMID   string              `json:"pmid,omitempty"`
	Status CitationMatchStatus `json:"status"`
}

// CitationMatches is the result of [Client.MatchCitations].
type CitationMatches struct {
	Matches []CitationMatch `json:"matches"`
}

// Found returns only the citations that resolved to a single PMID.
func (c *CitationMatches) Found() []CitationMatch {
	var found []CitationMatch
	for _, match := range c.Matches {
		if match.Status == CitationFound {
			found = append(found, match)
		}
	}
	return found
}

// OASubsetInfo describes an article's PMC Open Access status, as returned by
// [Client.IsOASubset].
type OASubsetInfo struct {
	PMCID string `json:"pmcid"`
	// IsOASubset reports whether the article's files can be downloaded
	// programmatically. PMC's web site shows many articles this is false for.
	IsOASubset bool   `json:"is_oa_subset"`
	Citation   string `json:"citation,omitempty"`
	// License is the licence identifier, e.g. "CC BY".
	License string `json:"license,omitempty"`
	// Retracted reports whether PMC has marked the article as retracted.
	Retracted bool `json:"retracted"`
	// DownloadLink and DownloadFormat describe the package NCBI offers.
	DownloadLink   string `json:"download_link,omitempty"`
	DownloadFormat string `json:"download_format,omitempty"`
	Updated        string `json:"updated,omitempty"`
	// ErrorCode and ErrorMessage carry NCBI's explanation when IsOASubset is
	// false.
	ErrorCode    string `json:"error_code,omitempty"`
	ErrorMessage string `json:"error_message,omitempty"`
}

// ImageDimensions is a figure's pixel size.
type ImageDimensions struct {
	Width  uint32
	Height uint32
}

// UnmarshalJSON decodes the [width, height] pair the Rust side emits.
func (d *ImageDimensions) UnmarshalJSON(data []byte) error {
	var pair [2]uint32
	if err := json.Unmarshal(data, &pair); err != nil {
		return err
	}
	d.Width, d.Height = pair[0], pair[1]
	return nil
}

// ExtractedFigure is a downloaded figure image paired with its metadata from
// the article XML, as returned by [Client.ExtractFigures].
type ExtractedFigure struct {
	// Figure is the metadata from the XML, including the caption.
	Figure Figure `json:"figure"`
	// Path is where the image was written.
	Path string `json:"extracted_file_path"`
	// FileSize is the size in bytes, when it could be determined.
	FileSize *uint64 `json:"file_size"`
	// Dimensions is the pixel size, when the image format reported one.
	Dimensions *ImageDimensions `json:"dimensions"`
}

// EuropePMCResult is one record from a Europe PMC search.
//
// Europe PMC is lenient about its own schema: `core` results return far more
// fields than are modelled here, and the set changes over time. Whatever is not
// named below is kept in Extra rather than dropped.
type EuropePMCResult struct {
	// ID is the record's identifier within its source database.
	ID string `json:"id"`
	// Source is the source database code: MED, PMC, PPR, AGR, CBA, PAT, …
	Source string `json:"source"`
	// EuropePMCID is the fully-qualified "SOURCE/ID" address.
	EuropePMCID string `json:"europe_pmc_id"`
	PMID        string `json:"pmid,omitempty"`
	PMCID       string `json:"pmcid,omitempty"`
	DOI         string `json:"doi,omitempty"`
	Title       string `json:"title,omitempty"`
	// AuthorString is the comma-separated author list Europe PMC provides.
	AuthorString string `json:"author_string,omitempty"`
	JournalTitle string `json:"journal_title,omitempty"`
	PubYear      string `json:"pub_year,omitempty"`
	// IsOpenAccess is Europe PMC's flag, "Y" or "N".
	IsOpenAccess string `json:"is_open_access,omitempty"`
	// Extra holds the fields Europe PMC returned that are not modelled above.
	Extra map[string]any `json:"extra,omitempty"`
}

// OpenAccess reports whether Europe PMC flagged the record as open access.
func (r EuropePMCResult) OpenAccess() bool {
	return strings.EqualFold(r.IsOpenAccess, "Y")
}

// EuropePMCSearchPage is one page of Europe PMC search results, as returned by
// [Client.EuropePMCSearchPage].
type EuropePMCSearchPage struct {
	// HitCount is the total number of matching records across all pages.
	HitCount uint64 `json:"hit_count"`
	// NextCursorMark is the cursor for the following page. Europe PMC keeps
	// returning the same value once the last page is reached, so a cursor equal
	// to the one just used means there are no more pages.
	NextCursorMark string `json:"next_cursor_mark,omitempty"`
	// Results are the records on this page.
	Results []EuropePMCResult `json:"results"`
}

// EuropePMCReference is a work cited by a Europe PMC record.
type EuropePMCReference struct {
	// Source and ID identify the cited record, when Europe PMC matched it to
	// one of its own; both are empty for an unmatched reference.
	Source string `json:"source,omitempty"`
	ID     string `json:"id,omitempty"`
	// CitationType is e.g. "JOURNAL ARTICLE".
	CitationType        string `json:"citation_type,omitempty"`
	Title               string `json:"title,omitempty"`
	AuthorString        string `json:"author_string,omitempty"`
	JournalAbbreviation string `json:"journal_abbreviation,omitempty"`
	PubYear             string `json:"pub_year,omitempty"`
	Volume              string `json:"volume,omitempty"`
	Issue               string `json:"issue,omitempty"`
	PageInfo            string `json:"page_info,omitempty"`
	PMID                string `json:"pmid,omitempty"`
	DOI                 string `json:"doi,omitempty"`
	// Extra holds the fields Europe PMC returned that are not modelled above.
	Extra map[string]any `json:"extra,omitempty"`
}

// EuropePMCCitation is an article citing a Europe PMC record.
type EuropePMCCitation struct {
	// ID and Source identify the citing record.
	ID     string `json:"id,omitempty"`
	Source string `json:"source,omitempty"`
	// CitationType is e.g. "JOURNAL ARTICLE".
	CitationType        string `json:"citation_type,omitempty"`
	Title               string `json:"title,omitempty"`
	AuthorString        string `json:"author_string,omitempty"`
	JournalAbbreviation string `json:"journal_abbreviation,omitempty"`
	PubYear             string `json:"pub_year,omitempty"`
	Volume              string `json:"volume,omitempty"`
	Issue               string `json:"issue,omitempty"`
	PageInfo            string `json:"page_info,omitempty"`
	// CitedByCount is how many times the citing article has itself been cited.
	CitedByCount string `json:"cited_by_count,omitempty"`
	// Extra holds the fields Europe PMC returned that are not modelled above.
	Extra map[string]any `json:"extra,omitempty"`
}

// EuropePMCDBCrossReference is one entry in a database cross-reference group.
//
// Europe PMC documents the four slots only positionally, and their meaning
// varies by database, so they are carried through as-is rather than renamed.
type EuropePMCDBCrossReference struct {
	Info1 string `json:"info1,omitempty"`
	Info2 string `json:"info2,omitempty"`
	Info3 string `json:"info3,omitempty"`
	Info4 string `json:"info4,omitempty"`
}

// EuropePMCDatabaseLink groups a record's cross-references to one external
// database, as returned by [Client.EuropePMCDatabaseLinks].
type EuropePMCDatabaseLink struct {
	// DBName is the external database, e.g. "UNIPROT", "EMBL", "PDB".
	DBName string `json:"db_name,omitempty"`
	// DBCount is the number of cross-references Europe PMC reports.
	DBCount uint32 `json:"db_count,omitempty"`
	// Info holds the individual cross-reference entries.
	Info []EuropePMCDBCrossReference `json:"info"`
}
