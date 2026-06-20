/// Metadata-related configuration options
#[derive(Debug, Clone)]
pub struct MetadataOptions {
    /// Include metadata section at the top
    pub include_metadata: bool,
    /// Use YAML frontmatter for metadata instead of bold markdown format
    pub use_yaml_frontmatter: bool,
    /// Include author ORCID links
    pub include_orcid_links: bool,
    /// Include DOI and PMID links
    pub include_identifier_links: bool,
}

impl Default for MetadataOptions {
    fn default() -> Self {
        Self {
            include_metadata: true,
            use_yaml_frontmatter: false,
            include_orcid_links: true,
            include_identifier_links: true,
        }
    }
}

/// Figure and table display options
#[derive(Debug, Clone)]
pub struct FigureOptions {
    /// Include figure and table captions
    pub include_figure_captions: bool,
    /// Include local figure file paths in markdown images
    pub include_local_figures: bool,
}

impl Default for FigureOptions {
    fn default() -> Self {
        Self {
            include_figure_captions: true,
            include_local_figures: false,
        }
    }
}

/// Configuration options for Markdown conversion
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    /// Metadata display options
    pub metadata: MetadataOptions,
    /// Figure and table display options
    pub figures: FigureOptions,
    /// Include table of contents
    pub include_toc: bool,
    /// Heading style preference
    pub heading_style: HeadingStyle,
    /// Reference formatting style
    pub reference_style: ReferenceStyle,
    /// Maximum heading level (1-6)
    pub max_heading_level: u8,
}

/// Heading style options
#[derive(Debug, Clone, PartialEq)]
pub enum HeadingStyle {
    /// ATX style headers (# ## ###)
    ATX,
    /// Setext style headers (underlined)
    Setext,
}

/// Reference formatting style
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceStyle {
    /// Numbered references \[1\], \[2\], etc.
    Numbered,
    /// Author-year style (Smith, 2023)
    AuthorYear,
    /// Full citation format
    FullCitation,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            metadata: MetadataOptions::default(),
            figures: FigureOptions::default(),
            include_toc: false,
            heading_style: HeadingStyle::ATX,
            reference_style: ReferenceStyle::Numbered,
            max_heading_level: 6,
        }
    }
}
