package pubmedclient

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// A stub E-utilities server, so the offline suite can exercise the whole chain
// (Go -> cgo -> Rust -> HTTP -> parsing -> JSON -> Go structs) without touching
// NCBI. The payloads below are trimmed versions of real NCBI responses; each
// carries just enough to prove the field mapping.

const esearchResponse = `{"esearchresult":{"count":"2","retmax":"2","retstart":"0","idlist":["31978945","33515491"]}}`

const efetchResponse = `<?xml version="1.0" encoding="UTF-8"?>
<PubmedArticleSet>
  <PubmedArticle>
    <MedlineCitation>
      <PMID Version="1">31978945</PMID>
      <Article PubModel="Print">
        <Journal>
          <ISSN IssnType="Electronic">1476-4687</ISSN>
          <JournalIssue CitedMedium="Internet">
            <Volume>578</Volume>
            <Issue>7793</Issue>
            <PubDate><Year>2020</Year><Month>Feb</Month></PubDate>
          </JournalIssue>
          <Title>Nature</Title>
          <ISOAbbreviation>Nature</ISOAbbreviation>
        </Journal>
        <ArticleTitle>A test article about CRISPR.</ArticleTitle>
        <Pagination><MedlinePgn>82-93</MedlinePgn></Pagination>
        <Abstract>
          <AbstractText>An abstract used by the Go binding tests.</AbstractText>
        </Abstract>
        <AuthorList CompleteYN="Y">
          <Author ValidYN="Y">
            <LastName>Doe</LastName>
            <ForeName>Jane</ForeName>
            <Initials>J</Initials>
          </Author>
          <Author ValidYN="Y">
            <LastName>Roe</LastName>
            <ForeName>Richard</ForeName>
            <Initials>R</Initials>
          </Author>
        </AuthorList>
        <Language>eng</Language>
        <PublicationTypeList>
          <PublicationType UI="D016428">Journal Article</PublicationType>
        </PublicationTypeList>
      </Article>
    </MedlineCitation>
    <PubmedData>
      <ArticleIdList>
        <ArticleId IdType="pubmed">31978945</ArticleId>
        <ArticleId IdType="doi">10.1038/s41586-020-0000-0</ArticleId>
      </ArticleIdList>
    </PubmedData>
  </PubmedArticle>
</PubmedArticleSet>`

const esummaryResponse = `{
  "result": {
    "uids": ["31978945"],
    "31978945": {
      "uid": "31978945",
      "title": "A test article about CRISPR.",
      "source": "Nature",
      "fulljournalname": "Nature",
      "authors": [{"name": "Doe J", "authtype": "Author"}, {"name": "Roe R", "authtype": "Author"}],
      "pubdate": "2020 Feb",
      "epubdate": "2020 Jan 24",
      "volume": "578",
      "issue": "7793",
      "pages": "82-93",
      "lang": ["eng"],
      "issn": "0028-0836",
      "essn": "1476-4687",
      "pubtype": ["Journal Article", "Review"],
      "articleids": [
        {"idtype": "pubmed", "value": "31978945"},
        {"idtype": "doi", "value": "10.1038/s41586-020-0000-0"},
        {"idtype": "pmc", "value": "PMC7092803"}
      ],
      "sortpubdate": "2020/02/20 00:00",
      "pmcrefcount": 42,
      "recordstatus": "PubMed - indexed for MEDLINE"
    }
  }
}`

// The linkname decides which ELink call this answers, so one payload with
// several linksetdbs serves related articles, PMC links and citations alike.
const elinkResponse = `{
  "linksets": [{
    "dbfrom": "pubmed",
    "ids": ["31978945"],
    "linksetdbs": [
      {"dbto": "pubmed", "linkname": "pubmed_pubmed", "links": ["33515491", "25760099"]},
      {"dbto": "pubmed", "linkname": "pubmed_pubmed_citedin", "links": ["33515491"]},
      {"dbto": "pmc", "linkname": "pubmed_pmc", "links": ["7092803"]}
    ]
  }]
}`

const einfoListResponse = `{"einforesult": {"dblist": ["pubmed", "pmc", "nuccore"]}}`

const einfoDbResponse = `{
  "einforesult": {
    "dbinfo": [{
      "dbname": "pubmed",
      "menuname": "PubMed",
      "description": "PubMed bibliographic record",
      "dbbuild": "Build-2024",
      "count": "36000000",
      "lastupdate": "2024/01/01 00:00",
      "fieldlist": [
        {"name": "TITL", "fullname": "Title", "description": "Words in title",
         "termcount": "1000", "isdate": "N", "isnumerical": "N",
         "singletoken": "N", "hierarchy": "N", "ishidden": "N"},
        {"name": "PDAT", "fullname": "Date - Publication", "description": "Date of publication",
         "termcount": "500", "isdate": "Y", "isnumerical": "N",
         "singletoken": "Y", "hierarchy": "N", "ishidden": "N"}
      ],
      "linklist": [
        {"name": "pubmed_pmc", "menu": "Free full text in PMC",
         "description": "Free full text articles in PMC", "dbto": "pmc"}
      ]
    }]
  }
}`

const espellResponse = `<?xml version="1.0" encoding="UTF-8"?>
<eSpellResult>
  <Database>pubmed</Database>
  <Query>asthmaa treetment</Query>
  <CorrectedQuery>asthma treatment</CorrectedQuery>
  <SpelledQuery>
    <Replaced>asthma</Replaced>
    <Original> </Original>
    <Replaced>treatment</Replaced>
  </SpelledQuery>
</eSpellResult>`

const egqueryResponse = `<?xml version="1.0" encoding="UTF-8"?>
<Result>
  <Term>asthma</Term>
  <eGQueryResult>
    <ResultItem><DbName>pubmed</DbName><MenuName>PubMed</MenuName><Count>234567</Count><Status>Ok</Status></ResultItem>
    <ResultItem><DbName>pmc</DbName><MenuName>PMC</MenuName><Count>89012</Count><Status>Ok</Status></ResultItem>
    <ResultItem><DbName>mesh</DbName><MenuName>MeSH</MenuName><Count>0</Count><Status>Ok</Status></ResultItem>
  </eGQueryResult>
</Result>`

// ECitMatch answers in pipe-delimited plain text, one line per citation, with
// the PMID in the last field — AMBIGUOUS when several matched, and empty when
// nothing did.
const ecitmatchResponse = `proc+natl+acad+sci+u+s+a|1991|88|3248|mann+bj|Art1|2014248
science|2000|1|1|nobody|Art2|
nature|1999|5|10|someone|Art3|AMBIGUOUS`

// EPost uploads a PMID list to the history server and answers with the session
// identifiers that FetchAllByPMIDs then pages through.
const epostResponse = `{"epostresult": {"webenv": "NCID_1_test", "querykey": "1"}}`

const pmcEfetchResponse = `<?xml version="1.0" encoding="UTF-8"?>
<pmc-articleset>
<article xmlns:xlink="http://www.w3.org/1999/xlink" article-type="research-article">
  <front>
    <journal-meta>
      <journal-title-group><journal-title>Test Journal</journal-title></journal-title-group>
      <issn pub-type="epub">1234-5678</issn>
      <publisher><publisher-name>Test Publisher</publisher-name></publisher>
    </journal-meta>
    <article-meta>
      <article-id pub-id-type="pmc">7906746</article-id>
      <article-id pub-id-type="pmid">33515491</article-id>
      <article-id pub-id-type="doi">10.1234/test.2021</article-id>
      <title-group><article-title>A full-text article for the Go tests</article-title></title-group>
      <contrib-group>
        <contrib contrib-type="author">
          <name><surname>Doe</surname><given-names>Jane</given-names></name>
        </contrib>
      </contrib-group>
      <volume>12</volume>
      <issue>3</issue>
      <abstract><p>A short abstract.</p></abstract>
      <kwd-group><kwd>testing</kwd><kwd>bindings</kwd></kwd-group>
    </article-meta>
  </front>
  <body>
    <sec sec-type="intro" id="s1">
      <title>Introduction</title>
      <p>The introduction paragraph.</p>
      <fig id="fig1">
        <label>Figure 1</label>
        <caption><p>A caption for the first figure.</p></caption>
        <graphic xlink:href="fig1.jpg"/>
      </fig>
    </sec>
    <sec sec-type="methods" id="s2">
      <title>Methods</title>
      <p>The methods paragraph.</p>
    </sec>
  </body>
  <back>
    <ref-list>
      <ref id="ref1">
        <element-citation publication-type="journal">
          <person-group person-group-type="author">
            <name><surname>Roe</surname><given-names>Richard</given-names></name>
          </person-group>
          <article-title>A cited article</article-title>
          <source>Another Journal</source>
          <year>2019</year>
          <volume>5</volume>
          <fpage>1</fpage>
          <lpage>10</lpage>
        </element-citation>
      </ref>
    </ref-list>
  </back>
</article>
</pmc-articleset>`

// stubConfig tailors the stub server for one test.
type stubConfig struct {
	// observe, when set, is called for every request before it is answered.
	observe func(*http.Request)
	// status, when non-zero, makes every request fail with that status.
	status int
}

func defaultStub() *stubConfig { return &stubConfig{} }

// pathHasSuffix reports whether a request targets the given E-utilities
// endpoint, ignoring whatever prefix the stub server is mounted at.
func pathHasSuffix(r *http.Request, suffix string) bool {
	return strings.HasSuffix(r.URL.Path, suffix)
}

// stubResponses maps an endpoint to its canned body and content type. EFetch is
// shared by PubMed and PMC, so the handler picks by the `db` parameter instead.
var stubResponses = []struct {
	suffix      string
	contentType string
	body        string
}{
	{"/esearch.fcgi", "application/json", esearchResponse},
	{"/esummary.fcgi", "application/json", esummaryResponse},
	{"/elink.fcgi", "application/json", elinkResponse},
	{"/espell.fcgi", "application/xml", espellResponse},
	{"/egquery.fcgi", "application/xml", egqueryResponse},
	{"/ecitmatch.cgi", "text/plain", ecitmatchResponse},
	{"/epost.fcgi", "application/json", epostResponse},
}

// newStubClient starts a stub E-utilities server and returns a client pointed at
// it. The server and the client are cleaned up when the test ends.
func newStubClient(t *testing.T, config *stubConfig) *Client {
	t.Helper()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if config.observe != nil {
			config.observe(r)
		}
		if config.status != 0 {
			http.Error(w, "stub failure", config.status)
			return
		}

		if pathHasSuffix(r, "/efetch.fcgi") {
			// PMC full text and PubMed metadata share the endpoint.
			if r.URL.Query().Get("db") == "pmc" {
				writeStub(w, "application/xml", pmcEfetchResponse)
			} else {
				writeStub(w, "application/xml", efetchResponse)
			}
			return
		}
		if pathHasSuffix(r, "/einfo.fcgi") {
			// Without a `db` parameter EInfo lists the databases instead of
			// describing one.
			if r.URL.Query().Get("db") == "" {
				writeStub(w, "application/json", einfoListResponse)
			} else {
				writeStub(w, "application/json", einfoDbResponse)
			}
			return
		}

		for _, response := range stubResponses {
			if pathHasSuffix(r, response.suffix) {
				writeStub(w, response.contentType, response.body)
				return
			}
		}
		http.NotFound(w, r)
	}))
	t.Cleanup(server.Close)

	client, err := New(&Config{
		BaseURL: server.URL,
		Tool:    "pubmed-client-go-tests",
		// Keep the token bucket from slowing the suite down.
		RateLimit: 100,
		Timeout:   30 * time.Second,
	})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	t.Cleanup(func() { _ = client.Close() })

	return client
}

func writeStub(w http.ResponseWriter, contentType, body string) {
	w.Header().Set("Content-Type", contentType)
	_, _ = w.Write([]byte(body))
}
