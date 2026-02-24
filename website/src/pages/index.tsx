import Link from "@docusaurus/Link"
import CodeBlock from "@theme/CodeBlock"
import Layout from "@theme/Layout"
import type React from "react"
import { useState } from "react"
import styles from "./index.module.css"

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type LangTab = { value: string; label: string }

type DocCard = {
  title: string
  description: string
  href?: string
  comingSoon?: boolean
}

type Package = {
  name: string
  language: string
  registry: string
  href: string
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

const installTabs: LangTab[] = [
  { value: "rust", label: "🦀 Rust" },
  { value: "node", label: "🟢 Node.js" },
  { value: "wasm", label: "🌐 WebAssembly" },
  { value: "python", label: "🐍 Python" },
]

const quickstartTabs: LangTab[] = [
  { value: "rust", label: "🦀 Rust" },
  { value: "node", label: "🟢 Node.js" },
  { value: "python", label: "🐍 Python" },
]

const installCommands: Record<string, string> = {
  rust: "cargo add pubmed-client",
  node: "npm install pubmed-client",
  wasm: "npm install pubmed-client-wasm",
  python: "pip install pubmed-client-py",
}

const quickstartCode: Record<string, { code: string; language: string }> = {
  rust: {
    code: `use pubmed_client::PubMedClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();

    let articles = client
        .search()
        .query("COVID-19 vaccine")
        .limit(5)
        .search_and_fetch(&client)
        .await?;

    for article in articles {
        println!("{}", article.title);
    }
    Ok(())
}`,
    language: "rust",
  },
  node: {
    code: `import { PubMedClient } from "pubmed-client";

const client = new PubMedClient();
const articles = await client.search("COVID-19 vaccine", 5);

for (const article of articles) {
  console.log(article.title);
}`,
    language: "typescript",
  },
  python: {
    code: `from pubmed_client_py import Client

client = Client()
articles = client.pubmed.search_and_fetch("COVID-19 vaccine", 5)

for article in articles:
    print(article.title)`,
    language: "python",
  },
}

const docCards: DocCard[] = [
  {
    title: "🦀 Rust",
    description: "Generated rustdoc for the core pubmed-client crate",
    href: "https://illumination-k.github.io/pubmed-client/rust/pubmed_client/",
  },
  {
    title: "🟢 Node.js",
    description:
      "TypeDoc API reference for the native Node.js bindings (pubmed-client npm package)",
    href: "https://illumination-k.github.io/pubmed-client/node/",
  },
  {
    title: "🐍 Python",
    description: "Sphinx docs for pubmed-client-py",
    href: "https://illumination-k.github.io/pubmed-client/python/",
  },
]

const packages: Package[] = [
  {
    name: "pubmed-client",
    language: "Rust",
    registry: "crates.io",
    href: "https://crates.io/crates/pubmed-client",
  },
  {
    name: "pubmed-client",
    language: "Node.js (native)",
    registry: "npm",
    href: "https://www.npmjs.com/package/pubmed-client",
  },
  {
    name: "pubmed-client-wasm",
    language: "WebAssembly",
    registry: "npm",
    href: "https://www.npmjs.com/package/pubmed-client-wasm",
  },
  {
    name: "pubmed-client-py",
    language: "Python",
    registry: "PyPI",
    href: "https://pypi.org/project/pubmed-client-py/",
  },
]

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

function TabBar({
  tabs,
  selected,
  onSelect,
}: {
  tabs: LangTab[]
  selected: string
  onSelect: (v: string) => void
}): React.JSX.Element {
  return (
    <div className={styles.tabBar}>
      {tabs.map(tab => (
        <button
          key={tab.value}
          type="button"
          className={`${styles.tab} ${selected === tab.value ? styles.tabActive : ""}`}
          onClick={() => onSelect(tab.value)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  )
}

function CodeSnippet({
  code,
  language,
}: { code: string; language: string }): React.JSX.Element {
  return (
    <div className={styles.codeBlockWrapper}>
      <CodeBlock language={language}>{code}</CodeBlock>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function Home(): React.JSX.Element {
  const [lang, setLang] = useState("rust")

  // When the user picks a lang that only exists in installTabs (e.g. "wasm"),
  // clamp the quickstart selection to the closest valid tab.
  const quickstartLang = quickstartTabs.some(t => t.value === lang) ? lang : quickstartTabs[0].value

  const handleLangChange = (v: string) => setLang(v)

  return (
    <Layout
      title="PubMed & PMC API Client"
      description="Type-safe PubMed and PMC API client for Rust, Node.js, WebAssembly, and Python"
    >
      <main>
        {/* Hero */}
        <section className={styles.hero}>
          <div className={styles.heroInner}>
            <h1 className={styles.heroTitle}>pubmed-client</h1>
            <p className={styles.heroTagline}>
              Type-safe PubMed &amp; PMC API client
              <br />
              for Rust, Node.js, WebAssembly, and Python
            </p>
            <div className={styles.heroButtons}>
              <a
                className="button button--primary button--lg"
                href="https://illumination-k.github.io/pubmed-client/rust/pubmed_client/"
              >
                Rust API Docs
              </a>
              <a
                className="button button--secondary button--lg"
                href="https://illumination-k.github.io/pubmed-client/node/"
              >
                Node.js API Docs
              </a>
              <Link
                className="button button--secondary button--lg"
                href="https://github.com/illumination-k/pubmed-client"
              >
                GitHub
              </Link>
            </div>
          </div>
        </section>

        {/* Installation */}
        <section className={styles.section}>
          <div className={styles.container}>
            <h2>Installation</h2>
            <p className={styles.sectionIntro}>Install via your language's package manager.</p>
            <TabBar tabs={installTabs} selected={lang} onSelect={handleLangChange} />
            <CodeSnippet code={installCommands[lang] ?? ""} language="bash" />
          </div>
        </section>

        {/* Quickstart */}
        <section className={`${styles.section} ${styles.sectionAlt}`}>
          <div className={styles.container}>
            <h2>Quickstart</h2>
            <p className={styles.sectionIntro}>
              Search PubMed and fetch article metadata in a few lines.
            </p>
            <TabBar tabs={quickstartTabs} selected={quickstartLang} onSelect={handleLangChange} />
            <CodeSnippet
              code={quickstartCode[quickstartLang]?.code ?? ""}
              language={quickstartCode[quickstartLang]?.language ?? "text"}
            />
          </div>
        </section>

        {/* API Documentation */}
        <section className={styles.section}>
          <div className={styles.container}>
            <h2>API Documentation</h2>
            <div className={styles.cardGrid}>
              {docCards.map(card =>
                card.href ? (
                  <a key={card.title} className={styles.card} href={card.href}>
                    <h3>{card.title}</h3>
                    <p>{card.description}</p>
                  </a>
                ) : (
                  <div key={card.title} className={`${styles.card} ${styles.cardDisabled}`}>
                    <h3>
                      {card.title}
                      {card.comingSoon && <span className={styles.badge}>Coming soon</span>}
                    </h3>
                    <p>{card.description}</p>
                  </div>
                )
              )}
            </div>
          </div>
        </section>

        {/* Packages */}
        <section className={`${styles.section} ${styles.sectionAlt}`}>
          <div className={styles.container}>
            <h2>Packages</h2>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Package</th>
                  <th>Language</th>
                  <th>Registry</th>
                </tr>
              </thead>
              <tbody>
                {packages.map(pkg => (
                  <tr key={`${pkg.name}-${pkg.language}`}>
                    <td>
                      <code>{pkg.name}</code>
                    </td>
                    <td>{pkg.language}</td>
                    <td>
                      <a href={pkg.href}>{pkg.registry}</a>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </main>
    </Layout>
  )
}
