package pubmedclient

import (
	"encoding/json"
	"errors"
)

// SearchQuery builds a PubMed query without hand-writing field tags.
//
//	query := pubmedclient.NewSearchQuery().
//		TitleOrAbstract("CRISPR").
//		MeshTerm("Gene Editing").
//		PublishedAfter(pubmedclient.Year(2020)).
//		ArticleType("Review").
//		Limit(20).
//		Sort(pubmedclient.SortPublicationDate)
//
//	articles, err := client.SearchAndFetchQuery(ctx, query)
//
// The builder does not assemble the query string itself: it records the calls
// made and replays them against the Rust `SearchQuery` when the query is used
// or [SearchQuery.Build] is called. Field tags therefore have exactly one
// implementation across every language binding, and a correction there reaches
// Go without a matching edit here.
//
// Methods return the receiver, so calls chain. A SearchQuery is not safe for
// concurrent modification.
type SearchQuery struct {
	ops []queryOp
}

// queryOp is one recorded builder call. The names match the Rust SearchQuery
// methods, and the payload holds whatever arguments that method takes.
type queryOp struct {
	name    string
	payload map[string]any
}

// MarshalJSON writes the operation as {"op": name, ...payload}.
//
// The payload is built per method rather than described by struct tags: the
// operations take genuinely different arguments, and `omitempty` on a shared
// struct would quietly drop a zero year or an empty term instead of letting the
// Rust side reject it.
func (o queryOp) MarshalJSON() ([]byte, error) {
	object := make(map[string]any, len(o.payload)+1)
	for key, value := range o.payload {
		object[key] = value
	}
	object["op"] = o.name
	return json.Marshal(object)
}

// Date is a publication date at year, month or day precision. Lower precision
// widens the match: a year-only date covers the whole year.
type Date struct {
	Year  uint32 `json:"year"`
	Month uint32 `json:"month,omitempty"`
	Day   uint32 `json:"day,omitempty"`
}

// Year makes a year-precision [Date].
func Year(year uint32) Date { return Date{Year: year} }

// YearMonth makes a month-precision [Date].
func YearMonth(year, month uint32) Date { return Date{Year: year, Month: month} }

// YearMonthDay makes a day-precision [Date].
func YearMonthDay(year, month, day uint32) Date { return Date{Year: year, Month: month, Day: day} }

// NewSearchQuery creates an empty query builder.
func NewSearchQuery() *SearchQuery {
	return &SearchQuery{}
}

// record appends an operation and returns the receiver for chaining.
func (q *SearchQuery) record(name string, payload map[string]any) *SearchQuery {
	q.ops = append(q.ops, queryOp{name: name, payload: payload})
	return q
}

func (q *SearchQuery) value(name, value string) *SearchQuery {
	return q.record(name, map[string]any{"value": value})
}

func (q *SearchQuery) values(name string, values []string) *SearchQuery {
	if values == nil {
		values = []string{}
	}
	return q.record(name, map[string]any{"values": values})
}

func (q *SearchQuery) flag(name string) *SearchQuery {
	return q.record(name, nil)
}

func (q *SearchQuery) span(name string, start Date, end *Date) *SearchQuery {
	payload := map[string]any{"start": start}
	if end != nil {
		payload["end"] = *end
	}
	return q.record(name, payload)
}

// --- free-text terms ---------------------------------------------------------

// Query adds a free-text term, searched across every field.
func (q *SearchQuery) Query(terms string) *SearchQuery { return q.value("query", terms) }

// Terms adds several free-text terms at once.
func (q *SearchQuery) Terms(terms []string) *SearchQuery { return q.values("terms", terms) }

// TitleContains requires a term in the title ([ti]).
func (q *SearchQuery) TitleContains(title string) *SearchQuery {
	return q.value("title_contains", title)
}

// AbstractContains requires a term in the abstract.
func (q *SearchQuery) AbstractContains(text string) *SearchQuery {
	return q.value("abstract_contains", text)
}

// TitleOrAbstract requires a term in the title or abstract ([tiab]).
func (q *SearchQuery) TitleOrAbstract(text string) *SearchQuery {
	return q.value("title_or_abstract", text)
}

// --- people and places -------------------------------------------------------

// Author requires an author ([au]).
func (q *SearchQuery) Author(author string) *SearchQuery { return q.value("author", author) }

// FirstAuthor requires a first author ([1au]).
func (q *SearchQuery) FirstAuthor(author string) *SearchQuery {
	return q.value("first_author", author)
}

// LastAuthor requires a last author ([lastau]).
func (q *SearchQuery) LastAuthor(author string) *SearchQuery {
	return q.value("last_author", author)
}

// Affiliation requires an institution in the affiliation field ([ad]).
func (q *SearchQuery) Affiliation(institution string) *SearchQuery {
	return q.value("affiliation", institution)
}

// ORCID requires an author ORCID ([auid]).
func (q *SearchQuery) ORCID(orcid string) *SearchQuery { return q.value("orcid", orcid) }

// --- source ------------------------------------------------------------------

// Journal requires a journal name ([ta]).
func (q *SearchQuery) Journal(journal string) *SearchQuery { return q.value("journal", journal) }

// JournalAbbreviation requires an ISO journal abbreviation ([ta]).
func (q *SearchQuery) JournalAbbreviation(abbreviation string) *SearchQuery {
	return q.value("journal_abbreviation", abbreviation)
}

// GrantNumber requires a grant number ([gr]).
func (q *SearchQuery) GrantNumber(grant string) *SearchQuery {
	return q.value("grant_number", grant)
}

// ISBN requires an ISBN.
func (q *SearchQuery) ISBN(isbn string) *SearchQuery { return q.value("isbn", isbn) }

// ISSN requires an ISSN.
func (q *SearchQuery) ISSN(issn string) *SearchQuery { return q.value("issn", issn) }

// --- MeSH --------------------------------------------------------------------

// MeshTerm requires a MeSH descriptor ([mh]).
func (q *SearchQuery) MeshTerm(term string) *SearchQuery { return q.value("mesh_term", term) }

// MeshTerms requires several MeSH descriptors.
func (q *SearchQuery) MeshTerms(terms []string) *SearchQuery {
	return q.values("mesh_terms", terms)
}

// MeshMajorTopic requires a MeSH descriptor as a major topic ([majr]).
func (q *SearchQuery) MeshMajorTopic(term string) *SearchQuery {
	return q.value("mesh_major_topic", term)
}

// MeshSubheading requires a MeSH subheading ([sh]).
func (q *SearchQuery) MeshSubheading(subheading string) *SearchQuery {
	return q.value("mesh_subheading", subheading)
}

// OrganismMesh requires an organism MeSH term, such as "Humans" or "Mice".
func (q *SearchQuery) OrganismMesh(organism string) *SearchQuery {
	return q.value("organism_mesh", organism)
}

// AgeGroup requires an age-group MeSH term, such as "Aged" or "Infant".
func (q *SearchQuery) AgeGroup(ageGroup string) *SearchQuery {
	return q.value("age_group", ageGroup)
}

// --- flags -------------------------------------------------------------------

// HumanStudiesOnly restricts results to human studies.
func (q *SearchQuery) HumanStudiesOnly() *SearchQuery { return q.flag("human_studies_only") }

// AnimalStudiesOnly restricts results to animal studies.
func (q *SearchQuery) AnimalStudiesOnly() *SearchQuery { return q.flag("animal_studies_only") }

// FreeFullTextOnly restricts results to articles with free full text.
func (q *SearchQuery) FreeFullTextOnly() *SearchQuery { return q.flag("free_full_text_only") }

// FullTextOnly restricts results to articles with full text of any kind.
func (q *SearchQuery) FullTextOnly() *SearchQuery { return q.flag("full_text_only") }

// PMCOnly restricts results to articles available in PMC.
func (q *SearchQuery) PMCOnly() *SearchQuery { return q.flag("pmc_only") }

// HasAbstract restricts results to articles with an abstract.
func (q *SearchQuery) HasAbstract() *SearchQuery { return q.flag("has_abstract") }

// --- classification ----------------------------------------------------------

// ArticleType requires a publication type ([pt]), such as "Review",
// "Clinical Trial" or "Randomized Controlled Trial". Names are matched
// case-insensitively; an unrecognised one fails at [SearchQuery.Build] with
// [ErrInvalidArgument] and lists the accepted values.
func (q *SearchQuery) ArticleType(articleType string) *SearchQuery {
	return q.value("article_type", articleType)
}

// ArticleTypes requires any of several publication types.
func (q *SearchQuery) ArticleTypes(types []string) *SearchQuery {
	return q.values("article_types", types)
}

// Language requires a language ([la]). Both full names ("english") and ISO
// 639-2 codes ("eng") are accepted; an unrecognised value is passed through
// rather than rejected.
func (q *SearchQuery) Language(language string) *SearchQuery {
	return q.value("language", language)
}

// CustomFilter appends a raw PubMed filter expression, for anything the builder
// does not cover.
func (q *SearchQuery) CustomFilter(filter string) *SearchQuery {
	return q.value("custom_filter", filter)
}

// --- dates -------------------------------------------------------------------

// PublishedInYear requires a publication year ([pdat]).
func (q *SearchQuery) PublishedInYear(year uint32) *SearchQuery {
	return q.record("published_in_year", map[string]any{"year": year})
}

// DateRange requires a publication year range. Pass 0 for endYear to leave the
// range open-ended.
func (q *SearchQuery) DateRange(startYear, endYear uint32) *SearchQuery {
	payload := map[string]any{"start": startYear}
	if endYear != 0 {
		payload["end"] = endYear
	}
	return q.record("date_range", payload)
}

// PublishedBetween requires a publication date range. Pass nil for end to leave
// the range open-ended.
func (q *SearchQuery) PublishedBetween(start Date, end *Date) *SearchQuery {
	return q.span("published_between", start, end)
}

// PublishedAfter requires a publication date at or after date.
func (q *SearchQuery) PublishedAfter(date Date) *SearchQuery {
	return q.record("published_after", map[string]any{"date": date})
}

// PublishedBefore requires a publication date at or before date.
func (q *SearchQuery) PublishedBefore(date Date) *SearchQuery {
	return q.record("published_before", map[string]any{"date": date})
}

// EntryDateBetween requires an Entrez entry date range ([edat]) — when PubMed
// indexed the record, which can lag publication considerably.
func (q *SearchQuery) EntryDateBetween(start Date, end *Date) *SearchQuery {
	return q.span("entry_date_between", start, end)
}

// ModificationDateBetween requires a record modification date range ([mdat]).
func (q *SearchQuery) ModificationDateBetween(start Date, end *Date) *SearchQuery {
	return q.span("modification_date_between", start, end)
}

// --- boolean composition -----------------------------------------------------

// And requires another query in addition to this one.
func (q *SearchQuery) And(other *SearchQuery) *SearchQuery {
	return q.record("and", map[string]any{"ops": other.operations()})
}

// Or accepts either this query or another.
func (q *SearchQuery) Or(other *SearchQuery) *SearchQuery {
	return q.record("or", map[string]any{"ops": other.operations()})
}

// Exclude rejects results matching another query (NOT).
func (q *SearchQuery) Exclude(other *SearchQuery) *SearchQuery {
	return q.record("exclude", map[string]any{"ops": other.operations()})
}

// Negate inverts the query built so far.
func (q *SearchQuery) Negate() *SearchQuery { return q.flag("negate") }

// Group parenthesises the query built so far, so later boolean operators apply
// to it as a whole.
func (q *SearchQuery) Group() *SearchQuery { return q.flag("group") }

// Optimize removes duplicate and empty terms and filters.
func (q *SearchQuery) Optimize() *SearchQuery { return q.flag("optimize") }

// --- execution hints ---------------------------------------------------------

// Limit caps how many results [Client.Search] and [Client.SearchAndFetchQuery]
// request. Without it they ask for 20, the Rust builder's default.
func (q *SearchQuery) Limit(limit int) *SearchQuery {
	return q.record("limit", map[string]any{"value": limit})
}

// Sort selects the result ordering [Client.Search] and
// [Client.SearchAndFetchQuery] request. Passing [SortDefault] records nothing,
// leaving the ordering to PubMed.
func (q *SearchQuery) Sort(sort SortOrder) *SearchQuery {
	if sort == SortDefault {
		return q
	}
	return q.value("sort", string(sort))
}

// --- building ----------------------------------------------------------------

// operations returns the recorded operations, tolerating a nil receiver so an
// empty sub-query composes like any other.
func (q *SearchQuery) operations() []queryOp {
	if q == nil {
		return []queryOp{}
	}
	if q.ops == nil {
		return []queryOp{}
	}
	return q.ops
}

// builtQuery is what the Rust replay hands back.
type builtQuery struct {
	Query string    `json:"query"`
	Limit int       `json:"limit"`
	Sort  SortOrder `json:"sort"`
}

// queryRequest is what the Rust replay receives.
type queryRequest struct {
	Ops      []queryOp `json:"ops"`
	Validate bool      `json:"validate,omitempty"`
}

// Build assembles the PubMed query string.
//
// Building does not validate: an empty query builds to an empty string, exactly
// as the Rust builder does. Call [SearchQuery.Validate] when that matters.
func (q *SearchQuery) Build() (string, error) {
	built, err := q.build("Build", false)
	if err != nil {
		return "", err
	}
	return built.Query, nil
}

// String returns the built query, or the empty string if it cannot be built.
// Use [SearchQuery.Build] where the error matters; this is for logging.
func (q *SearchQuery) String() string {
	query, err := q.Build()
	if err != nil {
		return ""
	}
	return query
}

// Validate reports whether the query is well formed: non-empty, with a sane
// limit, balanced parentheses, and within PubMed's length cap. Failures match
// [ErrInvalidQuery]; a rejected argument, such as an unknown article type,
// matches [ErrInvalidArgument] instead.
func (q *SearchQuery) Validate() error {
	_, err := q.build("Validate", true)
	return err
}

// resolve builds the query along with the limit and sort recorded on it.
func (q *SearchQuery) resolve(op string) (builtQuery, error) {
	return q.build(op, false)
}

func (q *SearchQuery) build(op string, validate bool) (builtQuery, error) {
	if q == nil {
		return builtQuery{}, argError(op, "query must not be nil")
	}

	encoded, err := marshalArg(op, "query", queryRequest{Ops: q.operations(), Validate: validate})
	if err != nil {
		return builtQuery{}, err
	}

	raw, err := ffiQueryBuild(encoded)
	if err != nil {
		return builtQuery{}, reop(err, op)
	}

	var built builtQuery
	if err := decode(op, raw, &built); err != nil {
		return builtQuery{}, err
	}
	return built, nil
}

// reop relabels an error with the operation the caller invoked, since several
// methods reach the query builder through one FFI entry point.
func reop(err error, op string) error {
	var typed *Error
	if !errors.As(err, &typed) {
		return err
	}
	relabelled := *typed
	relabelled.Op = op
	return &relabelled
}
