package pubmedclient

import "context"

// HeadingStyle selects how Markdown headings are written.
type HeadingStyle string

const (
	// HeadingATX writes "# Heading".
	HeadingATX HeadingStyle = "atx"
	// HeadingSetext underlines headings instead.
	HeadingSetext HeadingStyle = "setext"
)

// ReferenceStyle selects how the reference list is formatted.
type ReferenceStyle string

const (
	// ReferenceNumbered writes numbered references: [1], [2], …
	ReferenceNumbered ReferenceStyle = "numbered"
	// ReferenceAuthorYear writes author-year references: (Smith, 2023).
	ReferenceAuthorYear ReferenceStyle = "author-year"
	// ReferenceFullCitation writes each reference out in full.
	ReferenceFullCitation ReferenceStyle = "full-citation"
)

// MarkdownOptions tunes [Client.FetchMarkdownWithOptions]. The zero value
// reproduces [Client.FetchMarkdown].
//
// The booleans are pointers because several of them default to true: a plain
// bool could not tell "leave the default alone" apart from "turn this off".
// Use [Bool] to set one.
type MarkdownOptions struct {
	// IncludeMetadata writes a metadata block above the body. Default: true.
	IncludeMetadata *bool `json:"include_metadata,omitempty"`
	// YAMLFrontmatter writes that metadata as YAML frontmatter rather than bold
	// Markdown. Default: false.
	YAMLFrontmatter *bool `json:"yaml_frontmatter,omitempty"`
	// IncludeORCIDLinks links author names to their ORCID records.
	// Default: true.
	IncludeORCIDLinks *bool `json:"include_orcid_links,omitempty"`
	// IncludeIdentifierLinks links the DOI and PMID. Default: true.
	IncludeIdentifierLinks *bool `json:"include_identifier_links,omitempty"`
	// IncludeFigureCaptions writes figure and table captions. Default: true.
	IncludeFigureCaptions *bool `json:"include_figure_captions,omitempty"`
	// IncludeTOC writes a table of contents. Default: false.
	IncludeTOC *bool `json:"include_toc,omitempty"`
	// HeadingStyle selects the heading syntax. Default: [HeadingATX].
	HeadingStyle HeadingStyle `json:"heading_style,omitempty"`
	// ReferenceStyle selects the reference format.
	// Default: [ReferenceNumbered].
	ReferenceStyle ReferenceStyle `json:"reference_style,omitempty"`
	// MaxHeadingLevel caps how deep nested sections go, from 1 to 6.
	// Default: 6.
	MaxHeadingLevel int `json:"max_heading_level,omitempty"`
	// FigurePaths maps a figure id to a local file path. Supplying any entry
	// also switches on local figure rendering, so images point at the files
	// [Client.ExtractFigures] wrote rather than at PMC.
	FigurePaths map[string]string `json:"figure_paths,omitempty"`
}

// Bool returns a pointer to value, for the optional booleans in
// [MarkdownOptions].
func Bool(value bool) *bool {
	return &value
}

// FetchFullText retrieves the full text of a PMC article. The pmcid may be
// given with or without the "PMC" prefix.
//
// Full text is only available for articles in the PMC Open Access subset; an
// article outside it matches [ErrPMCNotAvailable]. Use
// [Client.CheckPMCAvailability] to test a PMID first.
func (c *Client) FetchFullText(ctx context.Context, pmcid string) (*PMCArticle, error) {
	const op = "FetchFullText"

	var article PMCArticle
	err := c.callJSON(ctx, op, &article, func(h handle, t token) (string, error) {
		return ffiFetchFullText(h, t, pmcid)
	})
	if err != nil {
		return nil, err
	}
	return &article, nil
}

// FetchXML retrieves the raw JATS XML for a PMC article, for callers that need
// detail the flattened [PMCArticle] does not carry.
func (c *Client) FetchXML(ctx context.Context, pmcid string) (string, error) {
	return c.call(ctx, "FetchXML", func(h handle, t token) (string, error) {
		return ffiFetchXML(h, t, pmcid)
	})
}

// FetchMarkdown retrieves a PMC article and renders it as Markdown.
func (c *Client) FetchMarkdown(ctx context.Context, pmcid string) (string, error) {
	return c.call(ctx, "FetchMarkdown", func(h handle, t token) (string, error) {
		return ffiFetchMarkdown(h, t, "FetchMarkdown", pmcid, "")
	})
}

// FetchMarkdownWithOptions is [Client.FetchMarkdown] with the rendering tuned.
func (c *Client) FetchMarkdownWithOptions(ctx context.Context, pmcid string, options MarkdownOptions) (string, error) {
	const op = "FetchMarkdown"

	encoded, err := marshalArg(op, "options", options)
	if err != nil {
		return "", err
	}

	return c.call(ctx, op, func(h handle, t token) (string, error) {
		return ffiFetchMarkdown(h, t, op, pmcid, encoded)
	})
}

// CheckPMCAvailability reports whether a PMID has PMC full text available,
// returning the PMCID when it does.
func (c *Client) CheckPMCAvailability(ctx context.Context, pmid string) (pmcid string, available bool, err error) {
	const op = "CheckPMCAvailability"

	// JSON `null` when unavailable, otherwise the PMCID as a JSON string.
	var result *string
	if err := c.callJSON(ctx, op, &result, func(h handle, t token) (string, error) {
		return ffiCheckPMCAvailability(h, t, pmid)
	}); err != nil {
		return "", false, err
	}
	if result == nil {
		return "", false, nil
	}
	return *result, true, nil
}

// IsOASubset reports whether a PMC article is in the Open Access subset, along
// with its licence, retraction status, and download location.
//
// PMC's web site shows many articles that the OA subset does not cover, so this
// is the check to make before [Client.DownloadFiles] or
// [Client.ExtractFigures].
func (c *Client) IsOASubset(ctx context.Context, pmcid string) (*OASubsetInfo, error) {
	const op = "IsOASubset"

	var info OASubsetInfo
	err := c.callJSON(ctx, op, &info, func(h handle, t token) (string, error) {
		return ffiIsOASubset(h, t, pmcid)
	})
	if err != nil {
		return nil, err
	}
	return &info, nil
}

// DownloadFiles downloads an Open Access article's files into outputDir,
// creating the directory if needed, and returns the paths written.
//
// Files come from the PMC Open Access cloud mirror, so this works only for
// articles [Client.IsOASubset] reports as available.
func (c *Client) DownloadFiles(ctx context.Context, pmcid, outputDir string) ([]string, error) {
	const op = "DownloadFiles"
	if outputDir == "" {
		return nil, argError(op, "outputDir must not be empty")
	}

	var files []string
	err := c.callJSON(ctx, op, &files, func(h handle, t token) (string, error) {
		return ffiDownloadFiles(h, t, pmcid, outputDir)
	})
	if err != nil {
		return nil, err
	}
	return files, nil
}

// ExtractFigures downloads an Open Access article's figure images into
// outputDir and pairs each with its caption from the XML.
//
// The returned paths feed [MarkdownOptions.FigurePaths] directly, keyed by
// [Figure.ID].
func (c *Client) ExtractFigures(ctx context.Context, pmcid, outputDir string) ([]ExtractedFigure, error) {
	const op = "ExtractFigures"
	if outputDir == "" {
		return nil, argError(op, "outputDir must not be empty")
	}

	var figures []ExtractedFigure
	err := c.callJSON(ctx, op, &figures, func(h handle, t token) (string, error) {
		return ffiExtractFigures(h, t, pmcid, outputDir)
	})
	if err != nil {
		return nil, err
	}
	return figures, nil
}

// ClearPMCCache drops every cached PMC response. It is a no-op unless
// [Config.Cache] is set.
func (c *Client) ClearPMCCache(ctx context.Context) error {
	_, err := c.call(ctx, "ClearPMCCache", func(h handle, t token) (string, error) {
		return ffiClearPMCCache(h, t)
	})
	return err
}
