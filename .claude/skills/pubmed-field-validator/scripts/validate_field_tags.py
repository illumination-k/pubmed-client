#!/usr/bin/env python3
"""
PubMed Field Tag Validator

This script validates PubMed search field tags against the official NCBI documentation
and provides recommendations for invalid or deprecated tags.

Usage:
    python validate_field_tags.py --tags "ti" "au" "organism"
    python validate_field_tags.py --file path/to/tags.txt
    python validate_field_tags.py --scan path/to/source/code/
"""

import argparse
import re
import sys
from pathlib import Path

# Validated field tags from NCBI PubMed documentation
# Source: https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags
VALID_FIELD_TAGS = {
    "ti": "Title",
    "tiab": "Title/Abstract",
    "au": "Author",
    "1au": "First Author",
    "lastau": "Last Author",
    "ad": "Affiliation",
    "ta": "Journal Title Abbreviation",
    "la": "Language",
    "pt": "Publication Type",
    "mh": "MeSH Terms",
    "majr": "MeSH Major Topic",
    "sh": "MeSH Subheading",
    "gr": "Grant Number",
    "auid": "Author Identifier (ORCID)",
    "pdat": "Publication Date",
    "edat": "Entry Date",
    "mdat": "Modification Date",
    "sb": "Subset",
    "si": "Secondary Source ID",
    "ps": "Personal Name as Subject",
    "mhda": "MeSH Date",
    "dp": "Date of Publication",
    "dcom": "Date Completed",
    "lr": "Last Revision Date",
    "is": "ISSN",
    "vi": "Volume",
    "ip": "Issue",
    "pg": "Pagination",
    "ptyp": "Publication Type",
    "lang": "Language (deprecated, use [la])",
    "jour": "Journal",
    "vol": "Volume",
}

# Known invalid or non-existent tags
INVALID_FIELD_TAGS = {
    "organism": "Use MeSH terms with [mh] instead",
    "Organism": "Use MeSH terms with [mh] instead",
    "Title": "Use short form [ti] instead of long form",
    "Author": "Use short form [au] instead of long form",
    "Abstract": "Use [tiab] for Title/Abstract search",
}

# Deprecated tags with recommended alternatives
DEPRECATED_TAGS = {
    "lang": "la",
}


class FieldTagValidator:
    """Validates PubMed search field tags."""

    def __init__(self) -> None:
        self.valid_tags = VALID_FIELD_TAGS
        self.invalid_tags = INVALID_FIELD_TAGS
        self.deprecated_tags = DEPRECATED_TAGS

    def validate_tag(self, tag: str) -> tuple[str, str]:
        """
        Validate a single field tag.

        Args:
            tag: Field tag to validate (with or without brackets)

        Returns:
            Tuple of (status, message) where status is one of:
            - "valid": Tag is valid
            - "deprecated": Tag is deprecated
            - "invalid": Tag is known to be invalid
            - "unknown": Tag is not recognized
        """
        # Remove brackets if present
        clean_tag = tag.strip("[]")

        if clean_tag in self.valid_tags:
            if clean_tag in self.deprecated_tags:
                alternative = self.deprecated_tags[clean_tag]
                return (
                    "deprecated",
                    f"[{clean_tag}] ({self.valid_tags[clean_tag]}) is deprecated. Use [{alternative}] instead.",
                )
            return ("valid", f"[{clean_tag}] ({self.valid_tags[clean_tag]}) is valid.")

        if clean_tag in self.invalid_tags:
            return ("invalid", f"[{clean_tag}] is invalid. {self.invalid_tags[clean_tag]}")

        return ("unknown", f"[{clean_tag}] is not recognized. Check official documentation.")

    def validate_tags(self, tags: list[str]) -> dict[str, list[tuple[str, str]]]:
        """
        Validate multiple field tags.

        Args:
            tags: List of field tags to validate

        Returns:
            Dictionary with keys: valid, deprecated, invalid, unknown
            Each value is a list of (tag, message) tuples
        """
        results: dict[str, list[tuple[str, str]]] = {
            "valid": [],
            "deprecated": [],
            "invalid": [],
            "unknown": [],
        }

        for tag in tags:
            status, message = self.validate_tag(tag)
            results[status].append((tag, message))

        return results

    def scan_file_for_tags(self, file_path: Path) -> set[str]:
        """
        Scan a source file for PubMed field tags.

        Args:
            file_path: Path to source file

        Returns:
            Set of field tags found in the file
        """
        # Pattern to match field tags in square brackets
        pattern = r"\[([a-zA-Z0-9]+)\]"
        tags = set()

        try:
            content = file_path.read_text(encoding="utf-8")
            matches = re.finditer(pattern, content)
            for match in matches:
                tag = match.group(1)
                # Only consider tags that could be PubMed field tags
                # (short lowercase/numeric strings)
                if len(tag) <= 10 and not tag.isupper():
                    tags.add(tag)
        except Exception as e:
            print(f"Warning: Could not read {file_path}: {e}", file=sys.stderr)

        return tags

    def scan_directory(
        self, directory: Path, extensions: list[str] | None = None
    ) -> set[str]:
        """
        Recursively scan directory for field tags.

        Args:
            directory: Directory to scan
            extensions: List of file extensions to scan (default: .rs, .py, .md)

        Returns:
            Set of field tags found across all files
        """
        if extensions is None:
            extensions = [".rs", ".py", ".md", ".toml"]

        all_tags = set()

        for ext in extensions:
            for file_path in directory.rglob(f"*{ext}"):
                tags = self.scan_file_for_tags(file_path)
                all_tags.update(tags)

        return all_tags


def print_results(results: dict[str, list[tuple[str, str]]]) -> None:
    """Print validation results in a formatted way."""
    print("\n" + "=" * 80)
    print("PubMed Field Tag Validation Results")
    print("=" * 80)

    if results["valid"]:
        print(f"\n✅ VALID TAGS ({len(results['valid'])}):")
        for _tag, message in results["valid"]:
            print(f"  {message}")

    if results["deprecated"]:
        print(f"\n⚠️  DEPRECATED TAGS ({len(results['deprecated'])}):")
        for _tag, message in results["deprecated"]:
            print(f"  {message}")

    if results["invalid"]:
        print(f"\n❌ INVALID TAGS ({len(results['invalid'])}):")
        for _tag, message in results["invalid"]:
            print(f"  {message}")

    if results["unknown"]:
        print(f"\n❓ UNKNOWN TAGS ({len(results['unknown'])}):")
        for _tag, message in results["unknown"]:
            print(f"  {message}")

    print("\n" + "=" * 80)
    print("Reference: https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags")
    print("=" * 80 + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate PubMed search field tags against official documentation"
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--tags", nargs="+", help="Field tags to validate (e.g., ti au organism)")
    group.add_argument("--file", type=Path, help="File containing tags (one per line)")
    group.add_argument("--scan", type=Path, help="Scan directory for field tags in source code")

    args = parser.parse_args()

    validator = FieldTagValidator()
    tags = []

    if args.tags:
        tags = args.tags
    elif args.file:
        try:
            tags = args.file.read_text().strip().split("\n")
            tags = [tag.strip() for tag in tags if tag.strip()]
        except Exception as e:
            print(f"Error reading file: {e}", file=sys.stderr)
            sys.exit(1)
    elif args.scan:
        if not args.scan.is_dir():
            print(f"Error: {args.scan} is not a directory", file=sys.stderr)
            sys.exit(1)
        print(f"Scanning directory: {args.scan}")
        tags = list(validator.scan_directory(args.scan))
        if not tags:
            print("No field tags found in source code.")
            sys.exit(0)

    results = validator.validate_tags(tags)
    print_results(results)

    # Exit with error if there are invalid or unknown tags
    if results["invalid"] or results["unknown"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
