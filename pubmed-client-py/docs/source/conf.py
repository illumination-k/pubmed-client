"""Sphinx configuration for pubmed-client-py documentation."""

project = "pubmed-client-py"
copyright = "2025, illumination-k"  # noqa: A001
author = "illumination-k"
release = "0.0.2"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.napoleon",
    "sphinx_autodoc_typehints",
    "sphinx.ext.viewcode",
    "myst_parser",
    "sphinx_copybutton",
]

html_theme = "furo"
html_theme_options = {
    "source_repository": "https://github.com/illumination-k/pubmed-client",
    "source_branch": "main",
    "source_directory": "pubmed-client-py/docs/source/",
}

# Autodoc settings
# Use "signature" to avoid duplicate attribute descriptions that arise when
# sphinx-autodoc-typehints re-documents PyO3 class attributes already covered
# by autodoc.
autodoc_typehints = "signature"
autodoc_member_order = "bysource"
autodoc_default_options = {
    "members": True,
    "undoc-members": True,
    "show-inheritance": True,
}

# Suppress known-harmless warnings:
#   myst.xref_missing  - relative links in README.md (../pubmed-client, etc.)
#                        that only make sense in the repo, not in Sphinx.
suppress_warnings = ["myst.xref_missing"]

# Napoleon (NumPy/Google-style docstrings)
napoleon_google_docstring = True
napoleon_numpy_docstring = False

# MyST parser settings
myst_enable_extensions = ["colon_fence"]

# Copy button settings
copybutton_prompt_text = r">>> |\.\.\. |\$ "
copybutton_prompt_is_regexp = True
