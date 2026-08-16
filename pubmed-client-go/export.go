package pubmedclient

// ExportFormat names a citation format.
type ExportFormat string

const (
	// FormatBibTeX is BibTeX, for LaTeX bibliographies.
	FormatBibTeX ExportFormat = "bibtex"
	// FormatRIS is RIS, understood by EndNote, Mendeley and Zotero.
	FormatRIS ExportFormat = "ris"
	// FormatCSLJSON is CSL-JSON, used by citeproc and pandoc.
	FormatCSLJSON ExportFormat = "csl-json"
	// FormatNBIB is MEDLINE/NBIB, PubMed's own download format.
	FormatNBIB ExportFormat = "nbib"
)

// ExportArticles renders articles as a citation document.
//
//	bibtex, err := pubmedclient.ExportArticles(articles, pubmedclient.FormatBibTeX)
//
// Formatting happens in Rust, so the output matches the other language
// bindings and the CLI exactly. Exporting no articles yields an empty document.
//
// Articles built by hand rather than fetched work too; only the fields the
// chosen format uses need to be set.
func ExportArticles(articles []Article, format ExportFormat) (string, error) {
	const op = "ExportArticles"
	if len(articles) == 0 {
		return "", nil
	}

	encoded, err := marshalArg(op, "articles", articles)
	if err != nil {
		return "", err
	}

	return ffiExportArticles(op, encoded, string(format))
}

// Export renders a single article as a citation document.
func (a *Article) Export(format ExportFormat) (string, error) {
	const op = "Export"
	if a == nil {
		return "", argError(op, "article must not be nil")
	}

	encoded, err := marshalArg(op, "article", []*Article{a})
	if err != nil {
		return "", err
	}

	return ffiExportArticles(op, encoded, string(format))
}

// ToBibTeX renders the article as a BibTeX entry.
func (a *Article) ToBibTeX() (string, error) { return a.Export(FormatBibTeX) }

// ToRIS renders the article in RIS format.
func (a *Article) ToRIS() (string, error) { return a.Export(FormatRIS) }

// ToCSLJSON renders the article as a one-element CSL-JSON array.
func (a *Article) ToCSLJSON() (string, error) { return a.Export(FormatCSLJSON) }

// ToNBIB renders the article in MEDLINE/NBIB format.
func (a *Article) ToNBIB() (string, error) { return a.Export(FormatNBIB) }
