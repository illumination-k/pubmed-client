use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use wiremock::matchers::{method, path};
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

/// Minimal OA Cloud listing: the JATS XML, one figure image, and a
/// supplementary PDF — `download_pmc_files` fetches all three, while
/// `get_pmc_figure_images` only wants the two the figure resolves through.
const OA_CLOUD_LISTING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
<Contents><Key>PMC7906746.1/PMC7906746.1.xml</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/gr1_lrg.png</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/supplement.pdf</Key><Size>1</Size></Contents>
</ListBucketResult>"#;

/// Minimal JATS article carrying a single figure.
const OA_ARTICLE_XML: &str = r#"<?xml version="1.0"?>
<article xmlns:xlink="http://www.w3.org/1999/xlink">
  <front>
    <article-meta>
      <title-group><article-title>A study of things</article-title></title-group>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Results</title>
      <fig id="fig1">
        <label>Figure 1</label>
        <caption><p>Things, plotted.</p></caption>
        <graphic xlink:href="gr1_lrg"/>
      </fig>
    </sec>
  </body>
</article>"#;

/// A 4x3 PNG, so the tool has real pixel dimensions to report.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x03, 0x08, 0x02, 0x00, 0x00, 0x00, 0x3b, 0x96, 0x39,
    0x91, 0x00, 0x00, 0x00, 0x10, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x47, 0x0c, 0x38, 0x39, 0x00, 0xf5, 0x31, 0x0b, 0xf5, 0x35, 0x7b, 0xfb, 0x82, 0x00, 0x00, 0x00,
    0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

async fn mock_oa_cloud() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(OA_CLOUD_LISTING))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/PMC7906746.1/PMC7906746.1.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(OA_ARTICLE_XML))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/PMC7906746.1/gr1_lrg.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(TINY_PNG))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/PMC7906746.1/supplement.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4".to_vec()))
        .mount(&server)
        .await;

    server
}

#[tokio::test]
async fn test_figure_images_tool_returns_the_image_bytes_inline() -> Result<()> {
    // The point of `get_pmc_figure_images` is that the bytes themselves reach
    // the client: unit tests cover how a blob is wrapped, this covers that an
    // image content block survives the trip over stdio alongside its metadata.
    let cloud = mock_oa_cloud().await;
    let cloud_url = cloud.uri();

    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--oa-cloud-base-url")
                    .arg(&cloud_url)
                    .arg("--tools")
                    .arg("figure-images");
            },
        ))?)
        .await?;

    let arguments = serde_json::json!({ "pmc_id": "PMC7906746" });
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_pmc_figure_images").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await?;

    let structured = result
        .structured_content
        .expect("get_pmc_figure_images should answer with structuredContent");
    assert_eq!(structured["pmc_id"], "PMC7906746");
    assert_eq!(structured["figure_count"], 1);
    assert_eq!(structured["included_count"], 1);
    assert_eq!(structured["included_bytes"], TINY_PNG.len());
    assert_eq!(structured["figures"][0]["id"], "fig1");
    assert_eq!(structured["figures"][0]["label"], "Figure 1");
    assert_eq!(structured["figures"][0]["mime_type"], "image/png");
    assert_eq!(structured["figures"][0]["width"], 4);
    assert_eq!(structured["figures"][0]["height"], 3);
    assert_eq!(structured["figures"][0]["included"], true);

    let image = match result.content.first() {
        Some(rmcp::model::ContentBlock::Image(image)) => image.clone(),
        other => panic!("expected an image content block, got: {other:?}"),
    };
    assert_eq!(image.mime_type, "image/png");
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        image.data.as_bytes(),
    )?;
    assert_eq!(decoded, TINY_PNG);

    Ok(())
}

#[tokio::test]
async fn test_figure_images_tool_reports_figures_over_the_byte_budget() -> Result<()> {
    // Over-budget figures must still be described, so the caller learns the
    // figure exists and can ask for it with a larger budget — silently dropping
    // it would look like the article has no figures.
    let cloud = mock_oa_cloud().await;
    let cloud_url = cloud.uri();

    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--oa-cloud-base-url")
                    .arg(&cloud_url)
                    .arg("--tools")
                    .arg("figure-images");
            },
        ))?)
        .await?;

    let arguments = serde_json::json!({ "pmc_id": "PMC7906746", "max_total_bytes": 1 });
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_pmc_figure_images").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await?;

    let structured = result
        .structured_content
        .expect("get_pmc_figure_images should answer with structuredContent");
    assert_eq!(structured["figure_count"], 1);
    assert_eq!(structured["included_count"], 0);
    assert_eq!(structured["figures"][0]["included"], false);
    assert!(
        structured["figures"][0]["omitted_reason"].is_string(),
        "an omitted figure should say why, got: {structured}"
    );
    assert!(
        result.content.is_empty(),
        "nothing should be inlined, got: {:?}",
        result.content
    );

    Ok(())
}

#[tokio::test]
async fn test_download_figures_tool_writes_the_images_and_returns_their_paths() -> Result<()> {
    let cloud = mock_oa_cloud().await;
    let cloud_url = cloud.uri();
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().to_string_lossy().to_string();

    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--oa-cloud-base-url")
                    .arg(&cloud_url)
                    .arg("--tools")
                    .arg("download-figures");
            },
        ))?)
        .await?;

    let arguments = serde_json::json!({ "pmc_id": "PMC7906746", "output_dir": output_path });
    let result = client
        .call_tool(
            CallToolRequestParams::new("download_pmc_figures").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await?;

    let structured = result
        .structured_content
        .expect("download_pmc_figures should answer with structuredContent");
    assert_eq!(structured["figure_count"], 1);
    assert_eq!(structured["figures"][0]["id"], "fig1");
    assert_eq!(structured["figures"][0]["caption"], "Things, plotted.");
    assert_eq!(structured["output_dir"], output_path);

    let file_path = structured["figures"][0]["file_path"]
        .as_str()
        .expect("a downloaded figure should report where it landed");
    assert_eq!(std::fs::read(file_path)?, TINY_PNG);

    // Only the figure is written: the article XML and the supplementary PDF are
    // part of the OA package, but a figure download is not a package download.
    let mut written: Vec<String> = std::fs::read_dir(output_dir.path())?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().to_string()))
        .collect::<Result<Vec<_>>>()?;
    written.sort();
    assert_eq!(written, vec!["gr1_lrg.png".to_string()]);

    Ok(())
}

#[tokio::test]
async fn test_download_tools_accept_an_object_storage_destination() -> Result<()> {
    // The bucket is never reachable here, so this cannot assert an upload. What
    // it does assert is that an `s3://` destination is accepted and routed to
    // object storage rather than being created as a directory named "s3:" next
    // to the server — the failure mode a plain path check would let through.
    let cloud = mock_oa_cloud().await;
    let cloud_url = cloud.uri();
    let cwd = std::env::current_dir()?;

    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run")
                    .arg("-p")
                    .arg("pubmed-mcp")
                    .arg("--quiet")
                    .arg("--")
                    .arg("--oa-cloud-base-url")
                    .arg(&cloud_url)
                    .arg("--tools")
                    .arg("download-figures");
            },
        ))?)
        .await?;

    let arguments = serde_json::json!({
        "pmc_id": "PMC7906746",
        "output_dir": "s3://pubmed-client-test-bucket/figures",
    });
    let result = client
        .call_tool(
            CallToolRequestParams::new("download_pmc_figures").with_arguments(
                arguments
                    .as_object()
                    .expect("arguments should be a JSON object")
                    .clone(),
            ),
        )
        .await;

    // Without credentials the upload fails; what matters is that it failed while
    // talking to S3, not while parsing the destination, and that nothing was
    // written locally.
    match result {
        Ok(response) => {
            let structured = response
                .structured_content
                .expect("download_pmc_figures should answer with structuredContent");
            assert_eq!(
                structured["output_dir"],
                "s3://pubmed-client-test-bucket/figures"
            );
        }
        Err(err) => {
            let message = err.to_string();
            assert!(
                !message.contains("Invalid output_dir"),
                "an s3:// URI must parse as a destination, got: {message}"
            );
        }
    }

    assert!(
        !cwd.join("s3:").exists(),
        "an s3:// destination must never be created as a local directory"
    );

    Ok(())
}
