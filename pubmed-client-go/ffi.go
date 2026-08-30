package pubmedclient

/*
#cgo CFLAGS: -I${SRCDIR}/include

// The static archive is built by `mise run go:build` into a per-platform
// directory, so a checkout can hold prebuilt archives for several targets at
// once. Override with CGO_LDFLAGS to link an archive kept elsewhere.
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/lib/linux_amd64
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/lib/linux_arm64
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/lib/darwin_amd64
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/lib/darwin_arm64
#cgo windows,amd64 LDFLAGS: -L${SRCDIR}/lib/windows_amd64

#cgo LDFLAGS: -lpubmed_client_go

// System libraries the Rust standard library and rustls need. TLS is pure-Rust
// (rustls + ring), so there is deliberately no -lssl/-lcrypto here.
#cgo linux LDFLAGS: -lm -ldl -lpthread
#cgo darwin LDFLAGS: -framework CoreFoundation -framework Security -framework SystemConfiguration
#cgo windows LDFLAGS: -lws2_32 -luserenv -lntdll -lbcrypt -lcrypt32 -lsecur32 -lncrypt

#include <stdlib.h>
#include "pubmed_client.h"
*/
import "C"

import (
	"unsafe"
)

// This file is the only place that talks to C. Everything above it works with
// Go types; everything below the boundary follows the ownership rules in
// include/pubmed_client.h — every string the Rust side hands back is freed here.

// handle is an owned *C.PubmedClient.
type handle = *C.PubmedClient

// token is a borrowed *C.PubmedCancel, or nil when the call cannot be
// cancelled. Ownership stays with the caller in client.go.
type token = *C.PubmedCancel

// --- allocation bookkeeping --------------------------------------------------

// cargs collects the C strings allocated for one call so they can be released
// together. Every wrapper below builds one, defers free, and passes the
// pointers straight to C.
type cargs []*C.char

// str allocates a C copy of s.
func (a *cargs) str(s string) *C.char {
	allocated := C.CString(s)
	*a = append(*a, allocated)
	return allocated
}

// opt allocates a C copy of s, mapping the empty string to NULL. Every C
// function that takes an optional string documents NULL as "unset".
func (a *cargs) opt(s string) *C.char {
	if s == "" {
		return nil
	}
	return a.str(s)
}

// free releases every string allocated through this cargs.
func (a *cargs) free() {
	for _, allocated := range *a {
		C.free(unsafe.Pointer(allocated))
	}
}

// --- cancellation ------------------------------------------------------------

// newToken allocates a cancellation token. It never returns nil.
func newToken() token {
	return C.pubmed_cancel_new()
}

// triggerToken fires a token, aborting the call using it. Safe from any
// goroutine, and safe after the call has already returned.
func triggerToken(t token) {
	C.pubmed_cancel_trigger(t)
}

// freeToken releases a token. It must not run while a call still holds it.
func freeToken(t token) {
	C.pubmed_cancel_free(t)
}

// --- client lifecycle --------------------------------------------------------

// newHandle creates a Rust client from a JSON config blob. An empty configJSON
// means "library defaults".
func newHandle(configJSON string) (handle, error) {
	var args cargs
	defer args.free()

	var cErr *C.char
	h := C.pubmed_client_new(args.opt(configJSON), &cErr)
	if h == nil {
		return nil, takeError("New", cErr)
	}
	return h, nil
}

// freeHandle releases a handle. Passing nil is a no-op.
func freeHandle(h handle) {
	C.pubmed_client_free(h)
}

// --- result and error transfer -----------------------------------------------

// takeError converts an owned error envelope from the Rust side into a Go
// error, freeing the C string. A nil message means the call failed without
// reporting why, which should not happen but must not produce a nil error.
func takeError(op string, cErr *C.char) error {
	if cErr == nil {
		return &Error{Op: op, Kind: KindUnknown, Message: "call failed without an error message"}
	}
	defer C.pubmed_string_free(cErr)
	return parseError(op, C.GoString(cErr))
}

// takeString converts an owned result string into a Go string, freeing the C
// string. The caller must have checked that out is non-nil.
func takeString(out *C.char) string {
	defer C.pubmed_string_free(out)
	return C.GoString(out)
}

// invoke runs one boundary call: it copies the result into Go memory and frees
// whichever side of the (result, error) pair the Rust call allocated.
func invoke(op string, fn func(outErr **C.char) *C.char) (string, error) {
	var cErr *C.char
	out := fn(&cErr)
	if out == nil {
		return "", takeError(op, cErr)
	}
	return takeString(out), nil
}

// --- one thin wrapper per exported C function -------------------------------
//
// Each wrapper allocates its arguments, calls through, and returns the raw
// result string. Decoding lives in pubmed.go / pmc.go / query.go / export.go.

func ffiSearchArticles(h handle, t token, query string, limit int, sort string) (string, error) {
	var args cargs
	defer args.free()
	cQuery, cSort := args.str(query), args.opt(sort)

	return invoke("SearchArticles", func(outErr **C.char) *C.char {
		return C.pubmed_search_articles(h, cQuery, C.size_t(limit), cSort, t, outErr)
	})
}

func ffiFetchArticle(h handle, t token, pmid string) (string, error) {
	var args cargs
	defer args.free()
	cPMID := args.str(pmid)

	return invoke("FetchArticle", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_article(h, cPMID, t, outErr)
	})
}

func ffiFetchArticles(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("FetchArticles", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_articles(h, cPMIDs, t, outErr)
	})
}

func ffiFetchAllByPMIDs(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("FetchAllByPMIDs", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_all_by_pmids(h, cPMIDs, t, outErr)
	})
}

func ffiSearchAndFetch(h handle, t token, query string, limit int, sort string) (string, error) {
	var args cargs
	defer args.free()
	cQuery, cSort := args.str(query), args.opt(sort)

	return invoke("SearchAndFetch", func(outErr **C.char) *C.char {
		return C.pubmed_search_and_fetch(h, cQuery, C.size_t(limit), cSort, t, outErr)
	})
}

func ffiSearchWithFullText(h handle, t token, query string, limit int) (string, error) {
	var args cargs
	defer args.free()
	cQuery := args.str(query)

	return invoke("SearchWithFullText", func(outErr **C.char) *C.char {
		return C.pubmed_search_with_full_text(h, cQuery, C.size_t(limit), t, outErr)
	})
}

func ffiFetchSummaries(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("FetchSummaries", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_summaries(h, cPMIDs, t, outErr)
	})
}

func ffiSearchAndFetchSummaries(h handle, t token, query string, limit int, sort string) (string, error) {
	var args cargs
	defer args.free()
	cQuery, cSort := args.str(query), args.opt(sort)

	return invoke("SearchAndFetchSummaries", func(outErr **C.char) *C.char {
		return C.pubmed_search_and_fetch_summaries(h, cQuery, C.size_t(limit), cSort, t, outErr)
	})
}

func ffiGetRelatedArticles(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("GetRelatedArticles", func(outErr **C.char) *C.char {
		return C.pubmed_get_related_articles(h, cPMIDs, t, outErr)
	})
}

func ffiGetPMCLinks(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("GetPMCLinks", func(outErr **C.char) *C.char {
		return C.pubmed_get_pmc_links(h, cPMIDs, t, outErr)
	})
}

func ffiGetCitations(h handle, t token, pmidsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMIDs := args.str(pmidsJSON)

	return invoke("GetCitations", func(outErr **C.char) *C.char {
		return C.pubmed_get_citations(h, cPMIDs, t, outErr)
	})
}

func ffiGetDatabaseList(h handle, t token) (string, error) {
	return invoke("GetDatabaseList", func(outErr **C.char) *C.char {
		return C.pubmed_get_database_list(h, t, outErr)
	})
}

func ffiGetDatabaseInfo(h handle, t token, database string) (string, error) {
	var args cargs
	defer args.free()
	cDatabase := args.str(database)

	return invoke("GetDatabaseInfo", func(outErr **C.char) *C.char {
		return C.pubmed_get_database_info(h, cDatabase, t, outErr)
	})
}

func ffiGlobalQuery(h handle, t token, term string) (string, error) {
	var args cargs
	defer args.free()
	cTerm := args.str(term)

	return invoke("GlobalQuery", func(outErr **C.char) *C.char {
		return C.pubmed_global_query(h, cTerm, t, outErr)
	})
}

func ffiMatchCitations(h handle, t token, citationsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cCitations := args.str(citationsJSON)

	return invoke("MatchCitations", func(outErr **C.char) *C.char {
		return C.pubmed_match_citations(h, cCitations, t, outErr)
	})
}

func ffiSpellCheck(h handle, t token, op, term, database string) (string, error) {
	var args cargs
	defer args.free()
	cTerm, cDatabase := args.str(term), args.opt(database)

	return invoke(op, func(outErr **C.char) *C.char {
		return C.pubmed_spell_check(h, cTerm, cDatabase, t, outErr)
	})
}

func ffiFetchFullText(h handle, t token, pmcid string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID := args.str(pmcid)

	return invoke("FetchFullText", func(outErr **C.char) *C.char {
		return C.pmc_fetch_full_text(h, cPMCID, t, outErr)
	})
}

func ffiFetchXML(h handle, t token, pmcid string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID := args.str(pmcid)

	return invoke("FetchXML", func(outErr **C.char) *C.char {
		return C.pmc_fetch_xml(h, cPMCID, t, outErr)
	})
}

func ffiFetchMarkdown(h handle, t token, op, pmcid, optionsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID, cOptions := args.str(pmcid), args.opt(optionsJSON)

	return invoke(op, func(outErr **C.char) *C.char {
		return C.pmc_fetch_markdown(h, cPMCID, cOptions, t, outErr)
	})
}

func ffiCheckPMCAvailability(h handle, t token, pmid string) (string, error) {
	var args cargs
	defer args.free()
	cPMID := args.str(pmid)

	return invoke("CheckPMCAvailability", func(outErr **C.char) *C.char {
		return C.pmc_check_availability(h, cPMID, t, outErr)
	})
}

func ffiIsOASubset(h handle, t token, pmcid string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID := args.str(pmcid)

	return invoke("IsOASubset", func(outErr **C.char) *C.char {
		return C.pmc_is_oa_subset(h, cPMCID, t, outErr)
	})
}

func ffiDownloadFiles(h handle, t token, pmcid, outputDir string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID, cDir := args.str(pmcid), args.str(outputDir)

	return invoke("DownloadFiles", func(outErr **C.char) *C.char {
		return C.pmc_download_files(h, cPMCID, cDir, t, outErr)
	})
}

func ffiExtractFigures(h handle, t token, pmcid, outputDir string) (string, error) {
	var args cargs
	defer args.free()
	cPMCID, cDir := args.str(pmcid), args.str(outputDir)

	return invoke("ExtractFigures", func(outErr **C.char) *C.char {
		return C.pmc_extract_figures(h, cPMCID, cDir, t, outErr)
	})
}

func ffiClearPMCCache(h handle, t token) (string, error) {
	return invoke("ClearPMCCache", func(outErr **C.char) *C.char {
		return C.pmc_clear_cache(h, t, outErr)
	})
}

// --- Europe PMC --------------------------------------------------------------

func ffiEuropePmcSearch(h handle, t token, query string, limit int, optionsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cQuery, cOptions := args.str(query), args.opt(optionsJSON)

	return invoke("EuropePMCSearch", func(outErr **C.char) *C.char {
		return C.europe_pmc_search(h, cQuery, C.size_t(limit), cOptions, t, outErr)
	})
}

func ffiEuropePmcSearchPage(h handle, t token, query, optionsJSON string) (string, error) {
	var args cargs
	defer args.free()
	cQuery, cOptions := args.str(query), args.opt(optionsJSON)

	return invoke("EuropePMCSearchPage", func(outErr **C.char) *C.char {
		return C.europe_pmc_search_page(h, cQuery, cOptions, t, outErr)
	})
}

func ffiEuropePmcFetchFullText(h handle, t token, id, source string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource := args.str(id), args.opt(source)

	return invoke("EuropePMCFetchFullText", func(outErr **C.char) *C.char {
		return C.europe_pmc_fetch_full_text(h, cID, cSource, t, outErr)
	})
}

func ffiEuropePmcFetchXML(h handle, t token, id, source string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource := args.str(id), args.opt(source)

	return invoke("EuropePMCFetchXML", func(outErr **C.char) *C.char {
		return C.europe_pmc_fetch_xml(h, cID, cSource, t, outErr)
	})
}

func ffiEuropePmcGetReferences(h handle, t token, id, source string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource := args.str(id), args.opt(source)

	return invoke("EuropePMCReferences", func(outErr **C.char) *C.char {
		return C.europe_pmc_get_references(h, cID, cSource, t, outErr)
	})
}

func ffiEuropePmcGetCitations(h handle, t token, id, source string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource := args.str(id), args.opt(source)

	return invoke("EuropePMCCitations", func(outErr **C.char) *C.char {
		return C.europe_pmc_get_citations(h, cID, cSource, t, outErr)
	})
}

func ffiEuropePmcGetDatabaseLinks(h handle, t token, id, source string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource := args.str(id), args.opt(source)

	return invoke("EuropePMCDatabaseLinks", func(outErr **C.char) *C.char {
		return C.europe_pmc_get_database_links(h, cID, cSource, t, outErr)
	})
}

func ffiEuropePmcDownloadSupplementaryFiles(h handle, t token, id, source, outputPath string) (string, error) {
	var args cargs
	defer args.free()
	cID, cSource, cPath := args.str(id), args.opt(source), args.str(outputPath)

	return invoke("EuropePMCDownloadSupplementaryFiles", func(outErr **C.char) *C.char {
		return C.europe_pmc_download_supplementary_files(h, cID, cSource, cPath, t, outErr)
	})
}

// ffiQueryBuild and ffiExportArticles need no client handle: both are pure
// functions on the Rust side.

func ffiQueryBuild(requestJSON string) (string, error) {
	var args cargs
	defer args.free()
	cRequest := args.str(requestJSON)

	return invoke("Build", func(outErr **C.char) *C.char {
		return C.pubmed_query_build(cRequest, outErr)
	})
}

func ffiExportArticles(op, articlesJSON, format string) (string, error) {
	var args cargs
	defer args.free()
	cArticles, cFormat := args.str(articlesJSON), args.str(format)

	return invoke(op, func(outErr **C.char) *C.char {
		return C.pubmed_export_articles(cArticles, cFormat, outErr)
	})
}

// Version returns the version of the underlying Rust pubmed-client crate.
func Version() string {
	// Statically allocated on the Rust side; must not be freed.
	return C.GoString(C.pubmed_client_version())
}
