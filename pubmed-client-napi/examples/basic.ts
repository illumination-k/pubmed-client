/**
 * Basic example of using the PubMed client
 *
 * Run with: npx ts-node examples/basic.ts
 * Or after building: node examples/basic.js
 */

import { PubMedClient, Config, MarkdownOptions } from "../index";

async function main() {
  // Create a client with default configuration
  const client = new PubMedClient();

  // Or create with custom configuration
  // const config: Config = {
  //   apiKey: process.env.NCBI_API_KEY,
  //   email: "your-email@example.com",
  //   tool: "my-app",
  //   timeoutSeconds: 30,
  // };
  // const client = PubMedClient.withConfig(config);

  console.log("=== PubMed Search Example ===\n");

  // Search for COVID-19 vaccine articles
  const articles = await client.search("COVID-19 vaccine", 3);

  console.log(`Found ${articles.length} articles:\n`);

  for (const article of articles) {
    console.log(`PMID: ${article.pmid}`);
    console.log(`Title: ${article.title}`);
    console.log(`Journal: ${article.journal}`);
    console.log(`Date: ${article.pubDate}`);
    console.log(`Authors: ${article.authors.map((a) => a.fullName).join(", ")}`);
    if (article.doi) {
      console.log(`DOI: ${article.doi}`);
    }
    if (article.pmcId) {
      console.log(`PMC ID: ${article.pmcId}`);
    }
    console.log("---\n");
  }

  // Fetch a single article by PMID
  console.log("=== Fetch Single Article ===\n");

  const singleArticle = await client.fetchArticle("31978945");
  console.log(`Title: ${singleArticle.title}`);
  console.log(`Abstract: ${singleArticle.abstractText?.substring(0, 200)}...`);
  console.log("");

  // Check PMC availability
  console.log("=== Check PMC Availability ===\n");

  const pmcId = await client.checkPmcAvailability("31978945");
  if (pmcId) {
    console.log(`PMC ID available: ${pmcId}`);

    // Fetch full-text from PMC
    console.log("\n=== Fetch PMC Full-Text ===\n");

    const fullText = await client.fetchPmcArticle(pmcId);
    console.log(`Title: ${fullText.title}`);
    console.log(`Sections: ${fullText.sections.length}`);
    console.log(`References: ${fullText.references.length}`);

    // Convert to Markdown
    console.log("\n=== Convert to Markdown ===\n");

    const markdownOptions: MarkdownOptions = {
      includeMetadata: true,
      useYamlFrontmatter: true,
      includeToc: true,
      includeFigureCaptions: true,
    };

    const markdown = await client.fetchPmcAsMarkdown(pmcId, markdownOptions);
    console.log("Markdown preview (first 500 chars):");
    console.log(markdown.substring(0, 500));
    console.log("...\n");
  } else {
    console.log("No PMC full-text available for this article");
  }
}

main().catch(console.error);
