package pubmedclient

import "context"

// Europe PMC (https://europepmc.org) is a complementary index to the NCBI
// E-utilities: it covers preprints (PPR), patents (PAT), Agricola (AGR) and
// Chinese Biological Abstracts (CBA) as well as PubMed (MED) and PMC, and needs
// no API key.
//
// Records are addressed by a source database plus an id. The methods below take
// both; source may be empty, in which case a "PMC"-prefixed id is read as a PMC
// record and anything else as a PubMed record. An id given in fully-qualified
// "SOURCE/ID" form (e.g. "PPR/PPR123456") wins over source.

// EuropePMCResultType selects how much detail a Europe PMC search returns.
type EuropePMCResultType string

const (
	// EuropePMCIDList returns identifiers only.
	EuropePMCIDList EuropePMCResultType = "idlist"
	// EuropePMCLite returns the core bibliographic fields. This is the default.
	EuropePMCLite EuropePMCResultType = "lite"
	// EuropePMCCore returns full metadata, including abstracts and citation
	// counts. The fields beyond those modelled land in
	// [EuropePMCResult.Extra].
	EuropePMCCore EuropePMCResultType = "core"
)

// EuropePMCSearchOptions tunes a Europe PMC search. The zero value selects the
// library defaults: lite results, 25 records per request, the first page, and
// Europe PMC's own ordering.
type EuropePMCSearchOptions struct {
	// ResultType selects the level of detail. Default: [EuropePMCLite].
	ResultType EuropePMCResultType `json:"result_type,omitempty"`
	// PageSize is the number of records per request, from 1 to 1000.
	// Default: 25.
	PageSize int `json:"page_size,omitempty"`
	// CursorMark is the page to fetch; "*" is the first page. Pass the
	// [EuropePMCSearchPage.NextCursorMark] of the previous page to continue.
	// Only [Client.EuropePMCSearchPage] reads it; a whole-result-set search
	// always starts from the first page.
	CursorMark string `json:"cursor_mark,omitempty"`
	// Sort is a Europe PMC sort expression, e.g. "P_PDATE_D desc" for newest
	// first or "CITED desc" for most cited. Empty keeps relevance order.
	Sort string `json:"sort,omitempty"`
}

// EuropePMCSearch searches Europe PMC across every source it indexes, following
// pages until limit records are collected or the result set is exhausted.
//
// The query uses Europe PMC's own syntax, e.g. "TITLE:CRISPR AND SRC:PPR".
func (c *Client) EuropePMCSearch(ctx context.Context, query string, limit int) ([]EuropePMCResult, error) {
	return c.EuropePMCSearchWithOptions(ctx, query, limit, EuropePMCSearchOptions{})
}

// EuropePMCSearchWithOptions is [Client.EuropePMCSearch] with the detail level,
// page size and ordering tuned.
func (c *Client) EuropePMCSearchWithOptions(ctx context.Context, query string, limit int, options EuropePMCSearchOptions) ([]EuropePMCResult, error) {
	const op = "EuropePMCSearch"
	if err := checkLimit(op, limit); err != nil {
		return nil, err
	}

	encoded, err := marshalArg(op, "options", options)
	if err != nil {
		return nil, err
	}

	var results []EuropePMCResult
	if err := c.callJSON(ctx, op, &results, func(h handle, t token) (string, error) {
		return ffiEuropePmcSearch(h, t, query, limit, encoded)
	}); err != nil {
		return nil, err
	}
	return results, nil
}

// EuropePMCSearchPage fetches a single page of search results, for callers
// paging through a large result set themselves.
//
// Pass the returned [EuropePMCSearchPage.NextCursorMark] as
// [EuropePMCSearchOptions.CursorMark] to fetch the following page. Europe PMC
// signals the end by returning the cursor it was given.
func (c *Client) EuropePMCSearchPage(ctx context.Context, query string, options EuropePMCSearchOptions) (*EuropePMCSearchPage, error) {
	const op = "EuropePMCSearchPage"

	encoded, err := marshalArg(op, "options", options)
	if err != nil {
		return nil, err
	}

	var page EuropePMCSearchPage
	if err := c.callJSON(ctx, op, &page, func(h handle, t token) (string, error) {
		return ffiEuropePmcSearchPage(h, t, query, encoded)
	}); err != nil {
		return nil, err
	}
	return &page, nil
}

// EuropePMCFetchFullText retrieves the full text of a Europe PMC record.
//
// Parsing into a [PMCArticle] requires a PMC id, so this supports PMC-sourced
// records only; use [Client.EuropePMCFetchXML] for other sources.
func (c *Client) EuropePMCFetchFullText(ctx context.Context, id, source string) (*PMCArticle, error) {
	const op = "EuropePMCFetchFullText"

	var article PMCArticle
	if err := c.callJSON(ctx, op, &article, func(h handle, t token) (string, error) {
		return ffiEuropePmcFetchFullText(h, t, id, source)
	}); err != nil {
		return nil, err
	}
	return &article, nil
}

// EuropePMCFetchXML retrieves the raw JATS XML for a Europe PMC record, for
// callers that need detail the flattened [PMCArticle] does not carry, or a
// source [Client.EuropePMCFetchFullText] cannot parse.
func (c *Client) EuropePMCFetchXML(ctx context.Context, id, source string) (string, error) {
	return c.call(ctx, "EuropePMCFetchXML", func(h handle, t token) (string, error) {
		return ffiEuropePmcFetchXML(h, t, id, source)
	})
}

// EuropePMCReferences lists the works a Europe PMC record cites, following
// pages until the reference list is exhausted.
func (c *Client) EuropePMCReferences(ctx context.Context, id, source string) ([]EuropePMCReference, error) {
	const op = "EuropePMCReferences"

	var references []EuropePMCReference
	if err := c.callJSON(ctx, op, &references, func(h handle, t token) (string, error) {
		return ffiEuropePmcGetReferences(h, t, id, source)
	}); err != nil {
		return nil, err
	}
	return references, nil
}

// EuropePMCCitations lists the articles citing a Europe PMC record.
//
// Coverage is broader than [Client.GetCitations], which sees only
// PubMed-indexed articles: this includes preprints and other non-PubMed
// sources.
func (c *Client) EuropePMCCitations(ctx context.Context, id, source string) ([]EuropePMCCitation, error) {
	const op = "EuropePMCCitations"

	var citations []EuropePMCCitation
	if err := c.callJSON(ctx, op, &citations, func(h handle, t token) (string, error) {
		return ffiEuropePmcGetCitations(h, t, id, source)
	}); err != nil {
		return nil, err
	}
	return citations, nil
}

// EuropePMCDatabaseLinks lists a record's cross-references to external
// biological databases (UniProt, EMBL, PDB, ChEBI, ArrayExpress, …).
//
// A record with no cross-references returns an empty slice, not an error.
func (c *Client) EuropePMCDatabaseLinks(ctx context.Context, id, source string) ([]EuropePMCDatabaseLink, error) {
	const op = "EuropePMCDatabaseLinks"

	var links []EuropePMCDatabaseLink
	if err := c.callJSON(ctx, op, &links, func(h handle, t token) (string, error) {
		return ffiEuropePmcGetDatabaseLinks(h, t, id, source)
	}); err != nil {
		return nil, err
	}
	return links, nil
}

// EuropePMCDownloadSupplementaryFiles downloads a record's supplementary
// materials to outputPath and returns the written path.
//
// Europe PMC serves them as a single ZIP archive; unpacking is left to the
// caller (archive/zip). Parent directories are created if needed.
func (c *Client) EuropePMCDownloadSupplementaryFiles(ctx context.Context, id, source, outputPath string) (string, error) {
	const op = "EuropePMCDownloadSupplementaryFiles"
	if outputPath == "" {
		return "", argError(op, "outputPath must not be empty")
	}

	var written string
	if err := c.callJSON(ctx, op, &written, func(h handle, t token) (string, error) {
		return ffiEuropePmcDownloadSupplementaryFiles(h, t, id, source, outputPath)
	}); err != nil {
		return "", err
	}
	return written, nil
}
