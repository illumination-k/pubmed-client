package pubmedclient

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
