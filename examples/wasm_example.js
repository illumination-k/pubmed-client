// Example Node.js usage of the WASM PubMed client
// This demonstrates how the WASM bindings would be used

const { WasmPubMedClient, WasmClientConfig } = require('../pkg/pubmed_client_rs.js');

async function main() {
    try {
        console.log('Initializing PubMed WASM Client...');

        // Create configuration
        const config = new WasmClientConfig();
        config.set_email("researcher@university.edu");
        config.set_tool("PubMed WASM Example");

        // Create client
        const client = WasmPubMedClient.with_config(config);

        console.log('Searching for articles...');

        // Search for articles
        const articles = await client.search_articles("covid-19 treatment", 5);
        console.log(`Found ${articles.length} articles:`);

        articles.forEach((article, index) => {
            console.log(`\n${index + 1}. ${article.title}`);
            console.log(`   Authors: ${article.authors.join(', ')}`);
            console.log(`   Journal: ${article.journal}`);
            console.log(`   Date: ${article.pub_date}`);
            if (article.doi) {
                console.log(`   DOI: ${article.doi}`);
            }
        });

        // Fetch a specific article
        if (articles.length > 0) {
            const firstArticle = articles[0];
            console.log(`\nFetching detailed information for PMID: ${firstArticle.pmid}`);

            const detailedArticle = await client.fetch_article(firstArticle.pmid);
            console.log('Detailed article information:');
            console.log(`Title: ${detailedArticle.title}`);
            console.log(`Abstract: ${detailedArticle.abstract_text ? detailedArticle.abstract_text.substring(0, 200) + '...' : 'No abstract available'}`);

            // Check for PMC full text
            const pmcAvailable = await client.check_pmc_availability(firstArticle.pmid);
            if (pmcAvailable) {
                console.log(`\nPMC full text available: ${pmcAvailable}`);

                try {
                    const fullText = await client.fetch_full_text(pmcAvailable);
                    console.log('Full text sections:');
                    fullText.sections.forEach((section, index) => {
                        if (index < 3) { // Show first 3 sections
                            console.log(`  ${section.section_type}: ${section.title || 'No title'}`);
                            console.log(`    Content: ${section.content.substring(0, 100)}...`);
                        }
                    });

                    // Convert to markdown
                    const markdown = client.convert_to_markdown(fullText);
                    console.log(`\nMarkdown conversion successful. Length: ${markdown.length} characters`);

                } catch (error) {
                    console.log(`Error fetching full text: ${error.message}`);
                }
            } else {
                console.log('No PMC full text available for this article');
            }
        }

    } catch (error) {
        console.error('Error:', error.message);
    }
}

// Type definitions for better development experience
/**
 * @typedef {Object} Article
 * @property {string} pmid - PubMed ID
 * @property {string} title - Article title
 * @property {string[]} authors - List of author names
 * @property {string} journal - Journal name
 * @property {string} pub_date - Publication date
 * @property {string} [doi] - DOI if available
 * @property {string[]} article_types - Article types
 * @property {string} [abstract_text] - Abstract text if available
 */

/**
 * @typedef {Object} FullText
 * @property {string} pmcid - PMC ID
 * @property {string} [pmid] - PubMed ID
 * @property {string} title - Article title
 * @property {Object[]} authors - Detailed author information
 * @property {Object} journal - Journal information
 * @property {string} pub_date - Publication date
 * @property {string} [doi] - DOI if available
 * @property {Object[]} sections - Article sections
 * @property {Object[]} references - Reference list
 * @property {string} [article_type] - Article type
 * @property {string[]} keywords - Keywords
 */

if (require.main === module) {
    main().catch(console.error);
}

module.exports = { main };
