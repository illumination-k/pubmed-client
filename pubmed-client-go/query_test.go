package pubmedclient

import (
	"errors"
	"strings"
	"testing"
)

// The builder is a pure function of its recorded operations, so these tests
// need no client and no server.

func build(t *testing.T, query *SearchQuery) string {
	t.Helper()

	built, err := query.Build()
	if err != nil {
		t.Fatalf("Build() failed: %v", err)
	}
	return built
}

func TestBuilderEmitsFieldTags(t *testing.T) {
	tests := []struct {
		name  string
		query *SearchQuery
		want  string
	}{
		{"plain term", NewSearchQuery().Query("covid-19"), "covid-19"},
		{"title", NewSearchQuery().TitleContains("vaccine"), "vaccine[ti]"},
		{"title or abstract", NewSearchQuery().TitleOrAbstract("CRISPR"), "CRISPR[tiab]"},
		{"author", NewSearchQuery().Author("Smith J"), "Smith J[au]"},
		{"first author", NewSearchQuery().FirstAuthor("Smith J"), "Smith J[1au]"},
		{"last author", NewSearchQuery().LastAuthor("Smith J"), "Smith J[lastau]"},
		{"journal", NewSearchQuery().Journal("Nature"), "Nature[ta]"},
		{"affiliation", NewSearchQuery().Affiliation("Harvard"), "Harvard[ad]"},
		{"grant", NewSearchQuery().GrantNumber("R01"), "R01[gr]"},
		{"mesh", NewSearchQuery().MeshTerm("Neoplasms"), "Neoplasms[mh]"},
		{"mesh major", NewSearchQuery().MeshMajorTopic("Neoplasms"), "Neoplasms[majr]"},
		{"mesh subheading", NewSearchQuery().MeshSubheading("therapy"), "therapy[sh]"},
		{"publication year", NewSearchQuery().PublishedInYear(2023), "2023[pdat]"},
		{"year range", NewSearchQuery().DateRange(2020, 2023), "2020:2023[pdat]"},
		{"open-ended year range", NewSearchQuery().DateRange(2020, 0), "2020:3000[pdat]"},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if got := build(t, test.query); got != test.want {
				t.Errorf("Build() = %q, want %q", got, test.want)
			}
		})
	}
}

func TestBuilderJoinsTermsAndFilters(t *testing.T) {
	query := NewSearchQuery().
		Query("covid-19").
		TitleContains("vaccine").
		PublishedInYear(2023)

	got := build(t, query)
	for _, want := range []string{"covid-19", "vaccine[ti]", "2023[pdat]"} {
		if !strings.Contains(got, want) {
			t.Errorf("Build() = %q, missing %q", got, want)
		}
	}
	if strings.Count(got, " AND ") != 2 {
		t.Errorf("Build() = %q, want two AND joins", got)
	}
}

func TestBuilderDatePrecision(t *testing.T) {
	tests := []struct {
		name  string
		query *SearchQuery
		want  string
	}{
		{
			"year only",
			NewSearchQuery().PublishedBetween(Year(2020), ptr(Year(2023))),
			"2020:2023[pdat]",
		},
		{
			"month precision",
			NewSearchQuery().PublishedBetween(YearMonth(2020, 3), ptr(YearMonth(2021, 6))),
			"2020/03:2021/06[pdat]",
		},
		{
			"day precision",
			NewSearchQuery().PublishedBetween(
				YearMonthDay(2020, 3, 15), ptr(YearMonthDay(2021, 6, 30))),
			"2020/03/15:2021/06/30[pdat]",
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if got := build(t, test.query); got != test.want {
				t.Errorf("Build() = %q, want %q", got, test.want)
			}
		})
	}
}

func TestBuilderOpenEndedDates(t *testing.T) {
	// A nil end leaves the range open, which the Rust builder spells with its
	// own sentinel year rather than an empty bound.
	got := build(t, NewSearchQuery().PublishedBetween(Year(2020), nil))
	if !strings.HasPrefix(got, "2020:") || !strings.HasSuffix(got, "[pdat]") {
		t.Errorf("Build() = %q, want an open-ended 2020 range", got)
	}
}

func TestBuilderComposesBooleanQueries(t *testing.T) {
	query := NewSearchQuery().
		Query("cancer").
		Or(NewSearchQuery().Query("tumor")).
		Exclude(NewSearchQuery().Query("review"))

	got := build(t, query)
	if !strings.Contains(got, "OR") {
		t.Errorf("Build() = %q, missing an OR", got)
	}
	if !strings.Contains(got, "NOT") {
		t.Errorf("Build() = %q, missing a NOT", got)
	}
}

func TestBuilderFlags(t *testing.T) {
	query := NewSearchQuery().
		Query("asthma").
		HumanStudiesOnly().
		FreeFullTextOnly().
		HasAbstract()

	got := build(t, query)
	if !strings.Contains(strings.ToLower(got), "humans") {
		t.Errorf("Build() = %q, missing the human-studies filter", got)
	}
	if !strings.Contains(got, "[sb]") && !strings.Contains(strings.ToLower(got), "free full text") {
		t.Errorf("Build() = %q, missing the free-full-text filter", got)
	}
}

func TestBuilderArticleTypes(t *testing.T) {
	got := build(t, NewSearchQuery().Query("asthma").ArticleTypes([]string{"Review", "RCT"}))

	if !strings.Contains(got, "Review[pt]") {
		t.Errorf("Build() = %q, missing Review[pt]", got)
	}
	if !strings.Contains(got, "Randomized Controlled Trial[pt]") {
		t.Errorf("Build() = %q, missing the RCT publication type", got)
	}
}

func TestBuilderRejectsUnknownArticleTypes(t *testing.T) {
	_, err := NewSearchQuery().ArticleType("blog post").Build()
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("Build() = %v, want ErrInvalidArgument", err)
	}

	var typed *Error
	if errors.As(err, &typed) {
		// The message must list what is accepted, since the names come from
		// PubMed rather than from this package.
		if !strings.Contains(typed.Message, "Review") {
			t.Errorf("message %q does not list the accepted types", typed.Message)
		}
		if typed.Op != "Build" {
			t.Errorf("Op = %q, want %q", typed.Op, "Build")
		}
	}
}

// An unrecognised language is passed through rather than rejected, matching the
// Rust builder's Language::Other fallback.
func TestBuilderAcceptsUnknownLanguages(t *testing.T) {
	if got := build(t, NewSearchQuery().Language("klingon")); !strings.Contains(got, "[la]") {
		t.Errorf("Build() = %q, want a language filter", got)
	}
	// An ISO 639-2 code resolves to the same filter as the full name.
	if got, want := build(t, NewSearchQuery().Language("eng")), "English[la]"; got != want {
		t.Errorf("Build() = %q, want %q", got, want)
	}
}

func TestBuilderCustomFilterPassesThrough(t *testing.T) {
	got := build(t, NewSearchQuery().Query("asthma").CustomFilter("loattrfree full text[sb]"))
	if !strings.Contains(got, "loattrfree full text[sb]") {
		t.Errorf("Build() = %q, missing the custom filter", got)
	}
}

func TestBuilderOptimizeRemovesDuplicates(t *testing.T) {
	query := NewSearchQuery().Query("cancer").Query("cancer").Optimize()
	if got := build(t, query); got != "cancer" {
		t.Errorf("Build() = %q, want the duplicate term removed", got)
	}
}

func TestValidateRejectsEmptyAndOversizedQueries(t *testing.T) {
	if err := NewSearchQuery().Validate(); !errors.Is(err, ErrInvalidQuery) {
		t.Errorf("Validate() on an empty query = %v, want ErrInvalidQuery", err)
	}
	if err := NewSearchQuery().Query("cancer").Validate(); err != nil {
		t.Errorf("Validate() on a valid query = %v, want nil", err)
	}
	if err := NewSearchQuery().Query("cancer").Limit(0).Validate(); !errors.Is(err, ErrInvalidQuery) {
		t.Errorf("Validate() with a zero limit = %v, want ErrInvalidQuery", err)
	}
}

// Building must not validate, so an empty query still builds — the Rust API
// behaves the same way and callers rely on it to assemble queries piecemeal.
func TestBuildDoesNotValidate(t *testing.T) {
	got, err := NewSearchQuery().Build()
	if err != nil {
		t.Fatalf("Build() on an empty query failed: %v", err)
	}
	if got != "" {
		t.Errorf("Build() = %q, want an empty string", got)
	}
}

func TestBuilderLimitAndSortAreCarried(t *testing.T) {
	query := NewSearchQuery().Query("cancer").Limit(42).Sort(SortFirstAuthor)

	built, err := query.resolve("test")
	if err != nil {
		t.Fatalf("resolve failed: %v", err)
	}
	if built.Limit != 42 {
		t.Errorf("Limit = %d, want 42", built.Limit)
	}
	if built.Sort != SortFirstAuthor {
		t.Errorf("Sort = %q, want %q", built.Sort, SortFirstAuthor)
	}
}

func TestBuilderDefaultsToTheRustLimit(t *testing.T) {
	built, err := NewSearchQuery().Query("cancer").resolve("test")
	if err != nil {
		t.Fatalf("resolve failed: %v", err)
	}
	if built.Limit != 20 {
		t.Errorf("default Limit = %d, want 20", built.Limit)
	}
	if built.Sort != SortDefault {
		t.Errorf("default Sort = %q, want the zero value", built.Sort)
	}
}

// SortDefault means "let PubMed decide", so it must record nothing rather than
// sending an empty sort parameter the Rust side would reject.
func TestSortDefaultRecordsNothing(t *testing.T) {
	query := NewSearchQuery().Query("cancer").Sort(SortDefault)
	if len(query.ops) != 1 {
		t.Errorf("Sort(SortDefault) recorded an operation: %v", query.ops)
	}

	built, err := query.resolve("test")
	if err != nil {
		t.Fatalf("resolve failed: %v", err)
	}
	if built.Sort != SortDefault {
		t.Errorf("Sort = %q, want the zero value", built.Sort)
	}
}

func TestBuilderRejectsUnknownSortOrders(t *testing.T) {
	_, err := NewSearchQuery().Query("cancer").Sort(SortOrder("sideways")).Build()
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("Build() = %v, want ErrInvalidArgument", err)
	}
}

func TestStringIsTheBuiltQuery(t *testing.T) {
	query := NewSearchQuery().TitleContains("vaccine")
	if got := query.String(); got != "vaccine[ti]" {
		t.Errorf("String() = %q, want %q", got, "vaccine[ti]")
	}

	// String swallows the error, so an unbuildable query reads as empty rather
	// than panicking in a log line.
	if got := NewSearchQuery().ArticleType("blog post").String(); got != "" {
		t.Errorf("String() on an invalid query = %q, want an empty string", got)
	}
}

func TestNilQueryIsRejected(t *testing.T) {
	var query *SearchQuery
	if _, err := query.Build(); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("Build() on a nil query = %v, want ErrInvalidArgument", err)
	}
}

// A nil sub-query composes as an empty one rather than panicking.
func TestNilSubQueryComposes(t *testing.T) {
	var empty *SearchQuery
	if got := build(t, NewSearchQuery().Query("cancer").And(empty)); got == "" {
		t.Error("Build() with a nil sub-query returned an empty string")
	}
}

func ptr[T any](value T) *T { return &value }
