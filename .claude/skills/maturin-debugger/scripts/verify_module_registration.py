#!/usr/bin/env python3
"""
Verify that PyO3 classes are properly registered in the #[pymodule] function.
This script searches for #[pyclass] definitions and checks if they're added to the module.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def extract_pyclass_names(file_path: Path) -> set[str]:
    """
    Extract all #[pyclass] names from a Rust file.

    Args:
        file_path: Path to the Rust source file

    Returns:
        Set of struct names that have #[pyclass] attribute
    """
    content = file_path.read_text()

    # Pattern to match #[pyclass] followed by a struct
    # Handles both:
    # #[pyclass]
    # pub struct Name
    # and
    # #[pyclass(name = "PythonName")]
    # pub struct RustName

    pyclass_pattern = r'#\[(?:gen_stub_pyclass\s*,?\s*)?pyclass(?:\([^)]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?struct\s+(\w+)'

    matches = re.finditer(pyclass_pattern, content)
    return {match.group(1) for match in matches}


def extract_module_additions(file_path: Path) -> set[str]:
    """
    Extract all m.add_class::<Type>()? calls from the #[pymodule] function.

    Args:
        file_path: Path to the Rust source file

    Returns:
        Set of type names added to the module
    """
    content = file_path.read_text()

    # Pattern to match m.add_class::<ClassName>()?
    addition_pattern = r'm\.add_class::<(\w+)>\(\)\?'

    matches = re.finditer(addition_pattern, content)
    return {match.group(1) for match in matches}


def check_module_registration(src_dir: Path) -> tuple[set[str], set[str], set[str]]:
    """
    Check all Rust files for unregistered #[pyclass] types.

    Args:
        src_dir: Path to the src/ directory

    Returns:
        Tuple of (all_pyclass_names, registered_names, unregistered_names)
    """
    # Find all .rs files
    rs_files = list(src_dir.rglob("*.rs"))

    all_pyclass_names = set()
    all_registered_names = set()

    # Extract all #[pyclass] definitions
    for rs_file in rs_files:
        pyclass_names = extract_pyclass_names(rs_file)
        all_pyclass_names.update(pyclass_names)

        # Check if this file has a #[pymodule] function
        module_additions = extract_module_additions(rs_file)
        all_registered_names.update(module_additions)

    unregistered = all_pyclass_names - all_registered_names

    return all_pyclass_names, all_registered_names, unregistered


def main() -> None:
    """Main verification workflow."""
    print("🔍 PyO3 Module Registration Verifier")
    print("=" * 60)

    # Find src directory
    src_dir = Path("src")
    if not src_dir.exists():
        print("❌ No src/ directory found. Run this from the package root.")
        sys.exit(1)

    print(f"📂 Scanning {src_dir}/ for Rust files...")

    all_classes, registered, unregistered = check_module_registration(src_dir)

    print(f"\n📊 Found {len(all_classes)} #[pyclass] types:")
    for cls in sorted(all_classes):
        print(f"   - {cls}")

    print(f"\n✅ {len(registered)} types registered in #[pymodule]:")
    for cls in sorted(registered):
        print(f"   - {cls}")

    if unregistered:
        print(f"\n❌ {len(unregistered)} types NOT registered:")
        for cls in sorted(unregistered):
            print(f"   - {cls}")

        print("\n🔧 To fix this, add these lines to your #[pymodule] function:")
        print()
        for cls in sorted(unregistered):
            print(f"   m.add_class::<{cls}>()?;")

        sys.exit(1)
    else:
        print("\n✅ All #[pyclass] types are properly registered!")
        sys.exit(0)


if __name__ == "__main__":
    main()
