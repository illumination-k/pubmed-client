use anyhow::Result;
use rmcp::{
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
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

    // Verify server info
    assert_eq!(peer_info.server_info.name, "pubmed-mcp");
    assert_eq!(peer_info.server_info.version, "0.1.0");

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
    assert_eq!(peer_info.server_info.name, "pubmed-mcp");

    // Get server capabilities through peer info
    assert!(
        peer_info.capabilities.tools.is_some(),
        "Server should support tools capability"
    );

    Ok(())
}
