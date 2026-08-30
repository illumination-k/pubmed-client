use anyhow::Result;
use rmcp::{
    ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;

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
