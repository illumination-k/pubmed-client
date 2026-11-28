use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_mcp_server_initialize() -> Result<()> {
    // Start the MCP server as a child process using stdio
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("-p").arg("pubmed-mcp").arg("--quiet");
            },
        ))?)
        .await?;

    // Get peer information to verify server initialized correctly
    let peer_info = client.peer_info().expect("Peer info should be available");

    // Verify server info. The version is sourced from CARGO_PKG_VERSION (the
    // unified workspace version), so assert against the same to stay bump-proof.
    let server_info = peer_info
        .server_info
        .as_ref()
        .expect("Server info should be available");
    assert_eq!(server_info.name, "pubmed-mcp");
    assert_eq!(server_info.version, env!("CARGO_PKG_VERSION"));

    // The icon travels inline, so a stdio client can render it offline.
    let icons = server_info.icons.as_deref().unwrap_or_default();
    assert_eq!(icons.len(), 1);
    assert!(icons[0].src.starts_with("data:image/svg+xml;base64,"));
    assert_eq!(icons[0].mime_type.as_deref(), Some("image/svg+xml"));

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_list_tools() -> Result<()> {
    // Start the MCP server
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("-p").arg("pubmed-mcp").arg("--quiet");
            },
        ))?)
        .await?;

    // List all available tools
    let tools = client.list_all_tools().await?;

    // Verify that we have at least 2 tools
    assert!(
        tools.len() >= 2,
        "Server should have at least 2 tools (search_pubmed and get_pmc_markdown)"
    );

    // Verify search_pubmed tool exists
    let has_search = tools.iter().any(|tool| tool.name == "search_pubmed");
    assert!(has_search, "search_pubmed tool should be available");

    // Verify get_pmc_markdown tool exists
    let has_markdown = tools.iter().any(|tool| tool.name == "get_pmc_markdown");
    assert!(has_markdown, "get_pmc_markdown tool should be available");

    // Verify search_pubmed tool has description
    let search_tool = tools
        .iter()
        .find(|tool| tool.name == "search_pubmed")
        .unwrap();
    assert!(
        search_tool.description.is_some(),
        "search_pubmed should have a description"
    );

    // Verify fetch_articles tool exists (issue #279)
    let has_articles = tools.iter().any(|tool| tool.name == "fetch_articles");
    assert!(has_articles, "fetch_articles tool should be available");

    // Verify get_pmc_markdown tool has description
    let markdown_tool = tools
        .iter()
        .find(|tool| tool.name == "get_pmc_markdown")
        .unwrap();
    assert!(
        markdown_tool.description.is_some(),
        "get_pmc_markdown should have a description"
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_tool_filtering() -> Result<()> {
    // Start the MCP server with only two tools enabled. This exercises the
    // per-instance ToolRouter wiring (`#[tool_handler(router = self.tool_router)]`):
    // if listing fell back to the static router, every tool would show up.
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--tools")
                    .arg("search,markdown");
            },
        ))?)
        .await?;

    let tools = client.list_all_tools().await?;
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    assert_eq!(
        tools.len(),
        2,
        "only the enabled tools should be listed, got: {names:?}"
    );
    assert!(names.contains(&"search_pubmed"));
    assert!(names.contains(&"get_pmc_markdown"));

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_capabilities() -> Result<()> {
    // Start the MCP server
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("-p").arg("pubmed-mcp").arg("--quiet");
            },
        ))?)
        .await?;

    // Get peer capabilities
    let peer_info = client.peer_info().expect("Peer info should be available");

    // Verify server info
    let server_info = peer_info
        .server_info
        .as_ref()
        .expect("Server info should be available");
    assert_eq!(server_info.name, "pubmed-mcp");

    // Get server capabilities through peer info
    assert!(
        peer_info.capabilities.tools.is_some(),
        "Server should support tools capability"
    );

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_lists_europe_pmc_tools() -> Result<()> {
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("-p").arg("pubmed-mcp").arg("--quiet");
            },
        ))?)
        .await?;

    let tools = client.list_all_tools().await?;
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

    for expected in [
        "europe_pmc_search",
        "europe_pmc_fulltext",
        "europe_pmc_references",
        "europe_pmc_citations",
        "europe_pmc_database_links",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} should be available, got: {names:?}"
        );
        let tool = tools.iter().find(|tool| tool.name == expected).unwrap();
        assert!(
            tool.description.is_some(),
            "{expected} should have a description"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_europe_pmc_tool_filtering() -> Result<()> {
    // The `--tools` CLI values must map onto the registered tool names; a typo
    // in `ToolName::as_str` would silently leave the router empty here.
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--tools")
                    .arg("europe-pmc-search,europe-pmc-fulltext,europe-pmc-references,europe-pmc-citations,europe-pmc-database-links");
            },
        ))?)
        .await?;

    let tools = client.list_all_tools().await?;
    let mut names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "europe_pmc_citations",
            "europe_pmc_database_links",
            "europe_pmc_fulltext",
            "europe_pmc_references",
            "europe_pmc_search",
        ]
    );

    Ok(())
}

#[tokio::test]
async fn test_every_tool_advertises_an_output_schema() -> Result<()> {
    // Every tool answers with structured content, so the schema describing it
    // has to survive the trip to the client: without it, a client cannot
    // validate `structuredContent` and falls back to reading prose.
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("-p").arg("pubmed-mcp").arg("--quiet");
            },
        ))?)
        .await?;

    let tools = client.list_all_tools().await?;
    assert!(!tools.is_empty(), "server should register tools");

    for tool in &tools {
        let schema = tool
            .output_schema
            .as_ref()
            .unwrap_or_else(|| panic!("{} should advertise an outputSchema", tool.name));
        assert_eq!(
            schema.get("type").and_then(|ty| ty.as_str()),
            Some("object"),
            "{}'s outputSchema should describe a JSON object",
            tool.name
        );
    }

    Ok(())
}

/// Minimal ESearch answer: one PMID, so `search_pubmed` proceeds to EFetch.
const ESEARCH_RESPONSE: &str = r#"{
    "esearchresult": {
        "count": "1",
        "retmax": "1",
        "retstart": "0",
        "idlist": ["31978945"]
    }
}"#;

/// Minimal EFetch answer for the PMID above.
const EFETCH_RESPONSE: &str = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">31978945</PMID>
            <Article>
                <Journal><Title>Journal of Things</Title></Journal>
                <ArticleTitle>A study of things</ArticleTitle>
                <Abstract><AbstractText>Things were studied.</AbstractText></Abstract>
                <AuthorList>
                    <Author>
                        <LastName>Doe</LastName>
                        <ForeName>Jane</ForeName>
                    </Author>
                </AuthorList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
</PubmedArticleSet>"#;

#[tokio::test]
async fn test_tool_call_returns_structured_content() -> Result<()> {
    // Drive a whole call over stdio against a stubbed E-utilities endpoint:
    // the unit tests cover the shape of the output type, this covers that the
    // shape actually reaches the client as `structuredContent`.
    let ncbi = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/esearch.fcgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ESEARCH_RESPONSE))
        .mount(&ncbi)
        .await;
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EFETCH_RESPONSE))
        .mount(&ncbi)
        .await;

    let base_url = ncbi.uri();
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--base-url")
                    .arg(&base_url)
                    .arg("--tools")
                    .arg("search");
            },
        ))?)
        .await?;

    let arguments = serde_json::json!({ "query": "things", "max_results": 1 });
    let result = client
        .call_tool(
            CallToolRequestParams::new("search_pubmed").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await?;

    let structured = result
        .structured_content
        .expect("search_pubmed should answer with structuredContent");
    assert_eq!(structured["count"], 1);
    assert_eq!(structured["articles"][0]["pmid"], "31978945");
    assert_eq!(structured["articles"][0]["title"], "A study of things");
    assert_eq!(
        structured["articles"][0]["abstract_preview"],
        "Things were studied."
    );

    // The same JSON is echoed as a text block, so a client that only reads
    // `content` still gets the whole answer rather than an empty result.
    let text = match result.content.first() {
        Some(rmcp::model::ContentBlock::Text(text)) => text.text.clone(),
        other => panic!("expected a text content block, got: {other:?}"),
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text)?,
        structured
    );

    Ok(())
}

#[tokio::test]
async fn test_as_of_bounds_the_search_and_is_reported() -> Result<()> {
    // `--as-of` end to end: the flag reaches ESearch as an Entrez-date bound,
    // the answer says which day the server runs as of, and the one tool that
    // cannot be bounded is not listed.
    let ncbi = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/esearch.fcgi"))
        .and(query_param("datetype", "crdt"))
        .and(query_param("mindate", "1000/01/01"))
        .and(query_param("maxdate", "2020/01/31"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ESEARCH_RESPONSE))
        .mount(&ncbi)
        .await;
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EFETCH_RESPONSE))
        .mount(&ncbi)
        .await;

    // Each hit's own dates are checked before it is fetched.
    Mock::given(method("GET"))
        .and(path("/esummary.fcgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"result":{"uids":["31978945"],"31978945":{"uid":"31978945","history":[{"pubstatus":"pubmed","date":"2020/01/25 06:00"},{"pubstatus":"entrez","date":"2020/01/25 06:00"}]}}}"#,
        ))
        .mount(&ncbi)
        .await;

    let base_url = ncbi.uri();
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--base-url")
                    .arg(&base_url)
                    // Set through the environment, as an MCP host config would.
                    .env("PUBMED_MCP_AS_OF", "2020-02-01");
            },
        ))?)
        .await?;

    let tools = client.list_all_tools().await?;
    assert!(tools.iter().any(|tool| tool.name == "search_pubmed"));
    assert!(
        !tools.iter().any(|tool| tool.name == "global_query"),
        "global_query cannot be bounded by date and must not be offered"
    );
    let instructions = client
        .peer_info()
        .and_then(|info| info.instructions.clone())
        .unwrap_or_default();
    assert!(instructions.contains("As of 2020-02-01"), "{instructions}");

    let arguments = serde_json::json!({ "query": "things", "max_results": 1 });
    let result = client
        .call_tool(
            CallToolRequestParams::new("search_pubmed").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await?;

    let structured = result
        .structured_content
        .expect("search_pubmed should answer with structuredContent");
    assert_eq!(structured["as_of"], "2020-02-01");
    assert_eq!(structured["count"], 1);
    assert!(
        structured["filters_applied"][0]
            .as_str()
            .is_some_and(|filter| filter.starts_with("As of 2020-02-01")),
        "{structured}"
    );

    Ok(())
}
