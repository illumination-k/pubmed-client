package pubmedclient

/*
#cgo CFLAGS: -I${SRCDIR}/include

// The static archive is built by `make build` (see the Makefile) into a
// per-platform directory, so a checkout can hold prebuilt archives for several
// targets at once. Override with CGO_LDFLAGS to link an archive kept elsewhere.
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

// newHandle creates a Rust client from a JSON config blob. An empty configJSON
// means "library defaults".
func newHandle(configJSON string) (handle, error) {
	var cErr *C.char

	var h handle
	if configJSON == "" {
		h = C.pubmed_client_new(nil, &cErr)
	} else {
		cConfig := C.CString(configJSON)
		defer C.free(unsafe.Pointer(cConfig))
		h = C.pubmed_client_new(cConfig, &cErr)
	}

	if h == nil {
		return nil, takeError("new", cErr)
	}
	return h, nil
}

// freeHandle releases a handle. Passing nil is a no-op.
func freeHandle(h handle) {
	C.pubmed_client_free(h)
}

// takeError converts an owned error string from the Rust side into a Go error,
// freeing the C string. A nil message means the call failed without reporting
// why, which should not happen but must not produce a nil error.
func takeError(op string, cErr *C.char) error {
	if cErr == nil {
		return &Error{Op: op, Message: "call failed without an error message"}
	}
	defer C.pubmed_string_free(cErr)
	return &Error{Op: op, Message: C.GoString(cErr)}
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

func ffiSearchArticles(h handle, query string, limit int) (string, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))

	return invoke("SearchArticles", func(outErr **C.char) *C.char {
		return C.pubmed_search_articles(h, cQuery, C.size_t(limit), outErr)
	})
}

func ffiFetchArticle(h handle, pmid string) (string, error) {
	cPMID := C.CString(pmid)
	defer C.free(unsafe.Pointer(cPMID))

	return invoke("FetchArticle", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_article(h, cPMID, outErr)
	})
}

func ffiFetchArticles(h handle, pmidsJSON string) (string, error) {
	cPMIDs := C.CString(pmidsJSON)
	defer C.free(unsafe.Pointer(cPMIDs))

	return invoke("FetchArticles", func(outErr **C.char) *C.char {
		return C.pubmed_fetch_articles(h, cPMIDs, outErr)
	})
}

func ffiSearchAndFetch(h handle, query string, limit int) (string, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))

	return invoke("SearchAndFetch", func(outErr **C.char) *C.char {
		return C.pubmed_search_and_fetch(h, cQuery, C.size_t(limit), outErr)
	})
}

func ffiFetchFullText(h handle, pmcid string) (string, error) {
	cPMCID := C.CString(pmcid)
	defer C.free(unsafe.Pointer(cPMCID))

	return invoke("FetchFullText", func(outErr **C.char) *C.char {
		return C.pmc_fetch_full_text(h, cPMCID, outErr)
	})
}

func ffiFetchMarkdown(h handle, pmcid string) (string, error) {
	cPMCID := C.CString(pmcid)
	defer C.free(unsafe.Pointer(cPMCID))

	return invoke("FetchMarkdown", func(outErr **C.char) *C.char {
		return C.pmc_fetch_markdown(h, cPMCID, outErr)
	})
}

func ffiCheckPMCAvailability(h handle, pmid string) (string, error) {
	cPMID := C.CString(pmid)
	defer C.free(unsafe.Pointer(cPMID))

	return invoke("CheckPMCAvailability", func(outErr **C.char) *C.char {
		return C.pmc_check_availability(h, cPMID, outErr)
	})
}

// Version returns the version of the underlying Rust pubmed-client crate.
func Version() string {
	// Statically allocated on the Rust side; must not be freed.
	return C.GoString(C.pubmed_client_version())
}
