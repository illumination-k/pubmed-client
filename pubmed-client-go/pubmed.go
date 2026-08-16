package pubmedclient

import "context"

// SortOrder controls how PubMed orders search results. The zero value means
// PubMed's own default, which is relevance for most queries.
type SortOrder string

const (
	// SortDefault leaves the ordering to PubMed.
	SortDefault SortOrder = ""
	// SortRelevance orders by PubMed's relevance score.
	SortRelevance SortOrder = "relevance"
	// SortPublicationDate orders by publication date, newest first.
	SortPublicationDate SortOrder = "pub_date"
	// SortFirstAuthor orders alphabetically by first author.
	SortFirstAuthor SortOrder = "author"
	// SortJournalName orders alphabetically by journal.
	SortJournalName SortOrder = "journal"
)

// SearchOptions tunes a search beyond the query and limit.
type SearchOptions struct {
	// Sort selects the result ordering. The zero value keeps PubMed's default.
	Sort SortOrder
}

// SearchArticles searches PubMed and returns up to limit matching PMIDs.
//
// The query accepts PubMed's full syntax, including field tags such as
// "cancer[ti] AND 2023[pdat]". Use [NewSearchQuery] to build one instead of
// assembling the tags by hand.
func (c *Client) SearchArticles(ctx context.Context, query string, limit int) ([]string, error) {
	return c.SearchArticlesWithOptions(ctx, query, limit, SearchOptions{})
}

// SearchArticlesWithOptions is [Client.SearchArticles] with a result ordering.
func (c *Client) SearchArticlesWithOptions(ctx context.Context, query string, limit int, options SearchOptions) ([]string, error) {
	const op = "SearchArticles"
	if err := checkLimit(op, limit); err != nil {
		return nil, err
	}

	var pmids []string
	err := c.callJSON(ctx, op, &pmids, func(h handle, t token) (string, error) {
		return ffiSearchArticles(h, t, query, limit, string(options.Sort))
	})
	if err != nil {
		return nil, err
	}
	return pmids, nil
}

// Search runs a query built with [NewSearchQuery], honouring the limit and sort
// order recorded on it.
func (c *Client) Search(ctx context.Context, query *SearchQuery) ([]string, error) {
	built, err := query.resolve("Search")
	if err != nil {
		return nil, err
	}
	return c.SearchArticlesWithOptions(ctx, built.Query, built.Limit, SearchOptions{Sort: built.Sort})
}

// FetchArticle fetches the full metadata for a single PMID.
//
// A PMID PubMed does not know matches [ErrNotFound].
func (c *Client) FetchArticle(ctx context.Context, pmid string) (*Article, error) {
	const op = "FetchArticle"

	var article Article
	err := c.callJSON(ctx, op, &article, func(h handle, t token) (string, error) {
		return ffiFetchArticle(h, t, pmid)
	})
	if err != nil {
		return nil, err
	}
	return &article, nil
}

// FetchArticles fetches metadata for several PMIDs in one batched request.
// Passing no PMIDs returns an empty slice without contacting NCBI.
//
// NCBI caps a single EFetch request, so prefer [Client.FetchAllByPMIDs] for
// lists in the thousands.
func (c *Client) FetchArticles(ctx context.Context, pmids []string) ([]Article, error) {
	return c.fetchArticleBatch(ctx, "FetchArticles", pmids, ffiFetchArticles)
}

// FetchAllByPMIDs fetches metadata for an arbitrarily large PMID list, using
// NCBI's history server to page through it. Passing no PMIDs returns an empty
// slice without contacting NCBI.
func (c *Client) FetchAllByPMIDs(ctx context.Context, pmids []string) ([]Article, error) {
	return c.fetchArticleBatch(ctx, "FetchAllByPMIDs", pmids, ffiFetchAllByPMIDs)
}

// fetchArticleBatch is the shared body of the PMID-list fetches.
func (c *Client) fetchArticleBatch(
	ctx context.Context,
	op string,
	pmids []string,
	call func(handle, token, string) (string, error),
) ([]Article, error) {
	if len(pmids) == 0 {
		return []Article{}, nil
	}

	encoded, err := marshalArg(op, "pmids", pmids)
	if err != nil {
		return nil, err
	}

	var articles []Article
	err = c.callJSON(ctx, op, &articles, func(h handle, t token) (string, error) {
		return call(h, t, encoded)
	})
	if err != nil {
		return nil, err
	}
	return articles, nil
}

// SearchAndFetch searches PubMed and fetches metadata for each hit, combining
// [Client.SearchArticles] and [Client.FetchArticles] into one call.
func (c *Client) SearchAndFetch(ctx context.Context, query string, limit int) ([]Article, error) {
	return c.SearchAndFetchWithOptions(ctx, query, limit, SearchOptions{})
}

// SearchAndFetchWithOptions is [Client.SearchAndFetch] with a result ordering.
func (c *Client) SearchAndFetchWithOptions(ctx context.Context, query string, limit int, options SearchOptions) ([]Article, error) {
	const op = "SearchAndFetch"
	if err := checkLimit(op, limit); err != nil {
		return nil, err
	}

	var articles []Article
	err := c.callJSON(ctx, op, &articles, func(h handle, t token) (string, error) {
		return ffiSearchAndFetch(h, t, query, limit, string(options.Sort))
	})
	if err != nil {
		return nil, err
	}
	return articles, nil
}

// SearchAndFetchQuery runs a query built with [NewSearchQuery] and fetches
// metadata for each hit.
func (c *Client) SearchAndFetchQuery(ctx context.Context, query *SearchQuery) ([]Article, error) {
	built, err := query.resolve("SearchAndFetch")
	if err != nil {
		return nil, err
	}
	return c.SearchAndFetchWithOptions(ctx, built.Query, built.Limit, SearchOptions{Sort: built.Sort})
}

// SearchWithFullText searches PubMed and attaches PMC full text to each hit
// that has it. [SearchFullTextResult.FullText] is nil for the rest, which is
// the common case: most PubMed articles are not in the PMC Open Access subset.
//
// This makes up to two extra requests per hit, so keep limit small.
func (c *Client) SearchWithFullText(ctx context.Context, query string, limit int) ([]SearchFullTextResult, error) {
	const op = "SearchWithFullText"
	if err := checkLimit(op, limit); err != nil {
		return nil, err
	}

	var results []SearchFullTextResult
	err := c.callJSON(ctx, op, &results, func(h handle, t token) (string, error) {
		return ffiSearchWithFullText(h, t, query, limit)
	})
	if err != nil {
		return nil, err
	}
	return results, nil
}

// --- ESummary ----------------------------------------------------------------

// FetchSummaries fetches lightweight ESummary records for several PMIDs.
// Passing no PMIDs returns an empty slice without contacting NCBI.
//
// Summaries carry bibliographic metadata without the abstract, MeSH terms or
// chemical list, so they are much cheaper than [Client.FetchArticles] when only
// titles, authors and dates are needed.
func (c *Client) FetchSummaries(ctx context.Context, pmids []string) ([]ArticleSummary, error) {
	const op = "FetchSummaries"
	if len(pmids) == 0 {
		return []ArticleSummary{}, nil
	}

	encoded, err := marshalArg(op, "pmids", pmids)
	if err != nil {
		return nil, err
	}

	var summaries []ArticleSummary
	err = c.callJSON(ctx, op, &summaries, func(h handle, t token) (string, error) {
		return ffiFetchSummaries(h, t, encoded)
	})
	if err != nil {
		return nil, err
	}
	return summaries, nil
}

// FetchSummary fetches the ESummary record for a single PMID.
func (c *Client) FetchSummary(ctx context.Context, pmid string) (*ArticleSummary, error) {
	summaries, err := c.FetchSummaries(ctx, []string{pmid})
	if err != nil {
		return nil, err
	}
	if len(summaries) == 0 {
		return nil, &Error{
			Op:      "FetchSummary",
			Kind:    KindArticleNotFound,
			Message: "no summary returned for PMID " + pmid,
		}
	}
	return &summaries[0], nil
}

// SearchAndFetchSummaries searches PubMed and fetches an ESummary record for
// each hit.
func (c *Client) SearchAndFetchSummaries(ctx context.Context, query string, limit int) ([]ArticleSummary, error) {
	return c.SearchAndFetchSummariesWithOptions(ctx, query, limit, SearchOptions{})
}

// SearchAndFetchSummariesWithOptions is [Client.SearchAndFetchSummaries] with a
// result ordering.
func (c *Client) SearchAndFetchSummariesWithOptions(ctx context.Context, query string, limit int, options SearchOptions) ([]ArticleSummary, error) {
	const op = "SearchAndFetchSummaries"
	if err := checkLimit(op, limit); err != nil {
		return nil, err
	}

	var summaries []ArticleSummary
	err := c.callJSON(ctx, op, &summaries, func(h handle, t token) (string, error) {
		return ffiSearchAndFetchSummaries(h, t, query, limit, string(options.Sort))
	})
	if err != nil {
		return nil, err
	}
	return summaries, nil
}

// --- ELink -------------------------------------------------------------------

// GetRelatedArticles finds articles PubMed considers related to the given
// PMIDs.
//
// The ELink APIs take numeric PMIDs rather than the strings the rest of the
// package uses, matching NCBI's own UID parameter.
func (c *Client) GetRelatedArticles(ctx context.Context, pmids []uint32) (*RelatedArticles, error) {
	const op = "GetRelatedArticles"

	var related RelatedArticles
	if err := c.callLink(ctx, op, pmids, &related, ffiGetRelatedArticles); err != nil {
		return nil, err
	}
	return &related, nil
}

// GetPMCLinks finds the PMC identifiers of articles with full text available.
func (c *Client) GetPMCLinks(ctx context.Context, pmids []uint32) (*PMCLinks, error) {
	const op = "GetPMCLinks"

	var links PMCLinks
	if err := c.callLink(ctx, op, pmids, &links, ffiGetPMCLinks); err != nil {
		return nil, err
	}
	return &links, nil
}

// GetCitations finds articles that cite the given PMIDs.
//
// Coverage is limited to citing articles indexed in PMC, so the count is a
// lower bound rather than a complete citation total.
func (c *Client) GetCitations(ctx context.Context, pmids []uint32) (*Citations, error) {
	const op = "GetCitations"

	var citations Citations
	if err := c.callLink(ctx, op, pmids, &citations, ffiGetCitations); err != nil {
		return nil, err
	}
	return &citations, nil
}

// callLink is the shared body of the ELink calls.
func (c *Client) callLink(
	ctx context.Context,
	op string,
	pmids []uint32,
	target any,
	call func(handle, token, string) (string, error),
) error {
	if len(pmids) == 0 {
		return argError(op, "at least one PMID is required")
	}

	encoded, err := marshalArg(op, "pmids", pmids)
	if err != nil {
		return err
	}

	return c.callJSON(ctx, op, target, func(h handle, t token) (string, error) {
		return call(h, t, encoded)
	})
}

// --- EInfo -------------------------------------------------------------------

// GetDatabaseList returns the names of every database NCBI exposes through the
// E-utilities.
func (c *Client) GetDatabaseList(ctx context.Context) ([]string, error) {
	const op = "GetDatabaseList"

	var databases []string
	err := c.callJSON(ctx, op, &databases, func(h handle, t token) (string, error) {
		return ffiGetDatabaseList(h, t)
	})
	if err != nil {
		return nil, err
	}
	return databases, nil
}

// GetDatabaseInfo describes one NCBI database: its record count, its searchable
// fields, and the databases it links to.
func (c *Client) GetDatabaseInfo(ctx context.Context, database string) (*DatabaseInfo, error) {
	const op = "GetDatabaseInfo"

	var info DatabaseInfo
	err := c.callJSON(ctx, op, &info, func(h handle, t token) (string, error) {
		return ffiGetDatabaseInfo(h, t, database)
	})
	if err != nil {
		return nil, err
	}
	return &info, nil
}

// --- EGQuery, ECitMatch, ESpell ----------------------------------------------

// GlobalQuery counts the records matching a term in every Entrez database.
func (c *Client) GlobalQuery(ctx context.Context, term string) (*GlobalQueryResults, error) {
	const op = "GlobalQuery"

	var results GlobalQueryResults
	err := c.callJSON(ctx, op, &results, func(h handle, t token) (string, error) {
		return ffiGlobalQuery(h, t, term)
	})
	if err != nil {
		return nil, err
	}
	return &results, nil
}

// MatchCitations resolves bibliographic citations to PMIDs in one batch.
//
// Each query carries a caller-chosen [CitationQuery.Key] that comes back on the
// matching [CitationMatch], so results can be paired up regardless of order.
func (c *Client) MatchCitations(ctx context.Context, citations []CitationQuery) (*CitationMatches, error) {
	const op = "MatchCitations"
	if len(citations) == 0 {
		return &CitationMatches{Matches: []CitationMatch{}}, nil
	}

	encoded, err := marshalArg(op, "citations", citations)
	if err != nil {
		return nil, err
	}

	var matches CitationMatches
	err = c.callJSON(ctx, op, &matches, func(h handle, t token) (string, error) {
		return ffiMatchCitations(h, t, encoded)
	})
	if err != nil {
		return nil, err
	}
	return &matches, nil
}

// SpellCheck asks PubMed for spelling suggestions on a search term.
func (c *Client) SpellCheck(ctx context.Context, term string) (*SpellCheckResult, error) {
	return c.spellCheck(ctx, "SpellCheck", term, "")
}

// SpellCheckDB is [Client.SpellCheck] against a database other than PubMed,
// such as "pmc" or "nuccore".
func (c *Client) SpellCheckDB(ctx context.Context, term, database string) (*SpellCheckResult, error) {
	const op = "SpellCheckDB"
	if database == "" {
		return nil, argError(op, "database must not be empty; use SpellCheck for PubMed")
	}
	return c.spellCheck(ctx, op, term, database)
}

func (c *Client) spellCheck(ctx context.Context, op, term, database string) (*SpellCheckResult, error) {
	var result SpellCheckResult
	err := c.callJSON(ctx, op, &result, func(h handle, t token) (string, error) {
		return ffiSpellCheck(h, t, op, term, database)
	})
	if err != nil {
		return nil, err
	}
	return &result, nil
}
