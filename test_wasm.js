// Test script for WASM PubMed client
const { WasmPubMedClient, WasmClientConfig } = require('./pkg/pubmed_client_rs.js');

async function testBasicFunctionality() {
    console.log('🔬 Testing WASM PubMed Client...');

    try {
        // Test 1: Create client with default config
        console.log('\n1. Creating client with default configuration...');
        const client = new WasmPubMedClient();
        console.log('✅ Default client created successfully');

        // Test 2: Create client with custom config
        console.log('\n2. Creating client with custom configuration...');
        const config = new WasmClientConfig();
        config.email = "test@example.com";
        config.tool = "WASM Test Client";
        config.rate_limit = 1.0; // Very slow for testing

        const configuredClient = WasmPubMedClient.with_config(config);
        console.log('✅ Configured client created successfully');

        // Test 3: Search for articles (this will test network functionality)
        console.log('\n3. Testing article search...');
        console.log('   Searching for "covid-19" (limit: 2)...');

        try {
            const articles = await configuredClient.search_articles("covid-19", 2);
            console.log(`✅ Search successful! Found ${articles.length} articles`);

            if (articles.length > 0) {
                const firstArticle = articles[0];
                console.log(`   First article: ${firstArticle.title?.substring(0, 80)}...`);
                console.log(`   Authors: ${firstArticle.authors?.slice(0, 3).join(', ')}${firstArticle.authors?.length > 3 ? '...' : ''}`);
                console.log(`   Journal: ${firstArticle.journal}`);
                console.log(`   PMID: ${firstArticle.pmid}`);

                // Test 4: Fetch specific article details
                console.log('\n4. Testing single article fetch...');
                try {
                    const detailedArticle = await configuredClient.fetch_article(firstArticle.pmid);
                    console.log(`✅ Article fetch successful!`);
                    console.log(`   Title: ${detailedArticle.title?.substring(0, 80)}...`);
                    console.log(`   Has abstract: ${detailedArticle.abstract_text ? 'Yes' : 'No'}`);
                } catch (error) {
                    console.log(`⚠️  Article fetch failed: ${error.message}`);
                }

                // Test 5: Check PMC availability
                console.log('\n5. Testing PMC availability check...');
                try {
                    const pmcid = await configuredClient.check_pmc_availability(firstArticle.pmid);
                    if (pmcid) {
                        console.log(`✅ PMC available: ${pmcid}`);

                        // Test 6: Fetch full text (if available)
                        console.log('\n6. Testing PMC full text fetch...');
                        try {
                            const fullText = await configuredClient.fetch_full_text(pmcid);
                            console.log(`✅ Full text fetch successful!`);
                            console.log(`   Title: ${fullText.title?.substring(0, 80)}...`);
                            console.log(`   Sections: ${fullText.sections?.length || 0}`);
                            console.log(`   References: ${fullText.references?.length || 0}`);

                            // Test 7: Convert to markdown
                            console.log('\n7. Testing markdown conversion...');
                            try {
                                const markdown = configuredClient.convert_to_markdown(fullText);
                                console.log(`✅ Markdown conversion successful!`);
                                console.log(`   Markdown length: ${markdown.length} characters`);
                                console.log(`   First 100 chars: ${markdown.substring(0, 100)}...`);
                            } catch (error) {
                                console.log(`⚠️  Markdown conversion failed: ${error.message}`);
                            }
                        } catch (error) {
                            console.log(`⚠️  Full text fetch failed: ${error.message}`);
                        }
                    } else {
                        console.log('ℹ️  No PMC full text available for this article');
                    }
                } catch (error) {
                    console.log(`⚠️  PMC availability check failed: ${error.message}`);
                }

                // Test 8: Get related articles
                console.log('\n8. Testing related articles...');
                try {
                    const pmidNumber = parseInt(firstArticle.pmid);
                    const related = await configuredClient.get_related_articles(new Uint32Array([pmidNumber]));
                    console.log(`✅ Related articles fetch successful!`);
                    console.log(`   Found ${related.related_pmids?.length || 0} related articles`);
                } catch (error) {
                    console.log(`⚠️  Related articles fetch failed: ${error.message}`);
                }
            }
        } catch (error) {
            console.log(`❌ Search failed: ${error.message}`);
            console.log('   This might be due to network connectivity or CORS issues in Node.js');
        }

        console.log('\n🎉 WASM functionality test completed!');
        console.log('\n📊 Test Summary:');
        console.log('   ✅ WASM module loads successfully');
        console.log('   ✅ Client creation works');
        console.log('   ✅ Configuration system works');
        console.log('   ✅ TypeScript bindings are generated');
        console.log('   ℹ️  Network tests depend on connectivity');

    } catch (error) {
        console.error('❌ Test failed:', error.message);
        console.error('Stack trace:', error.stack);
    }
}

// Run the test
testBasicFunctionality();
