/**
 * Simple example: Search PubMed and download PMC articles as Markdown
 *
 * Run with: npx ts-node examples/search-and-markdown.ts
 */

import { writeFileSync } from 'node:fs'
import { PubMedClient } from '../index'

async function main() {
  const client = new PubMedClient()

  // Search for articles
  const query = process.argv[2] || 'CRISPR gene editing'
  const limit = parseInt(process.argv[3] || '5', 10)

  console.log(`Searching for: "${query}" (limit: ${limit})\n`)

  const articles = await client.search(query, limit)

  console.log(`Found ${articles.length} articles\n`)

  // Find articles with PMC full-text
  for (const article of articles) {
    console.log(`[${article.pmid}] ${article.title.substring(0, 60)}...`)

    if (article.pmcId) {
      console.log(`  -> PMC available: ${article.pmcId}`)

      // Download as Markdown
      const markdown = await client.fetchPmcAsMarkdown(article.pmcId, {
        includeMetadata: true,
        useYamlFrontmatter: true,
      })

      const filename = `${article.pmcId}.md`
      writeFileSync(filename, markdown)
      console.log(`  -> Saved to: ${filename}`)
    } else {
      console.log(`  -> No PMC full-text`)
    }
    console.log('')
  }
}

main().catch(console.error)
