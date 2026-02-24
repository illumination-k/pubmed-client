import Link from "@docusaurus/Link"
import Layout from "@theme/Layout"
import type React from "react"
import styles from "./index.module.css"

type DocCard = {
  title: string
  description: string
  href?: string
  comingSoon?: boolean
}

const docCards: DocCard[] = [
  {
    title: "🦀 Rust",
    description: "Generated rustdoc for the core pubmed-client crate",
    href: "https://illumination-k.github.io/pubmed-client/rust/pubmed_client/",
  },
  {
    title: "🟢 Node.js",
    description: "TypeDoc API reference for the native Node.js bindings (pubmed-client npm package)",
    href: "https://illumination-k.github.io/pubmed-client/node/",
  },
  {
    title: "🐍 Python",
    description: "Sphinx docs for pubmed-client-py",
    comingSoon: true,
  },
]

type Package = {
  name: string
  language: string
  registry: string
  href: string
}

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

export default function Home(): React.JSX.Element {
  return (
    <Layout
      title="PubMed & PMC API Client"
      description="Fast, type-safe PubMed and PMC API client for Rust, Node.js, WebAssembly, and Python"
    >
      <main>
        {/* Hero */}
        <section className={styles.hero}>
          <div className={styles.heroInner}>
            <h1 className={styles.heroTitle}>pubmed-client</h1>
            <p className={styles.heroTagline}>
              Fast, type-safe PubMed &amp; PMC API client
              <br />
              for Rust, Node.js, WebAssembly, and Python
            </p>
            <div className={styles.heroButtons}>
              <a className="button button--primary button--lg" href="https://illumination-k.github.io/pubmed-client/rust/pubmed_client/">
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
