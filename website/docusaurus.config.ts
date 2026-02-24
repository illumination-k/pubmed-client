import type * as Preset from "@docusaurus/preset-classic"
import type { Config } from "@docusaurus/types"

const config: Config = {
  title: "pubmed-client",
  tagline: "Fast, type-safe PubMed & PMC API client for Rust, Node.js, WebAssembly, and Python",
  url: "https://illumination-k.github.io",
  baseUrl: "/pubmed-client/",
  organizationName: "illumination-k",
  projectName: "pubmed-client",
  onBrokenLinks: "warn",
  trailingSlash: true,

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: false,
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      defaultMode: "light",
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: "pubmed-client",
      items: [
        {
          href: "rust/pubmed_client/",
          label: "Rust API",
          position: "left",
        },
        {
          href: "https://github.com/illumination-k/pubmed-client",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "API Docs",
          items: [
            {
              label: "Rust (rustdoc)",
              href: "rust/pubmed_client/",
            },
          ],
        },
        {
          title: "Packages",
          items: [
            {
              label: "crates.io",
              href: "https://crates.io/crates/pubmed-client",
            },
            {
              label: "npm (Node.js)",
              href: "https://www.npmjs.com/package/pubmed-client",
            },
            {
              label: "npm (WASM)",
              href: "https://www.npmjs.com/package/pubmed-client-wasm",
            },
            {
              label: "PyPI",
              href: "https://pypi.org/project/pubmed-client-py/",
            },
          ],
        },
        {
          title: "Community",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/illumination-k/pubmed-client",
            },
            {
              label: "Issues",
              href: "https://github.com/illumination-k/pubmed-client/issues",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} illumination-k. Built with Docusaurus.`,
    },
  } satisfies Preset.ThemeConfig,
}

export default config
