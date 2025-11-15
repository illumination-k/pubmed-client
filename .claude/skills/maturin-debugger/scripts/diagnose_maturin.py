#!/usr/bin/env python3
"""
Diagnostic script for maturin and PyO3 development issues.
This script checks common problems with module exports and builds.
"""

import sys
import importlib
import subprocess
from pathlib import Path


def print_header(title):
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print('=' * 60)


def check_module_exports(module_name, expected_classes=None):
    """
    Check if a Python module exports expected classes.

    Args:
        module_name: Name of the module to import
        expected_classes: List of class names to check for (optional)
    """
    print_header(f"Checking Module Exports: {module_name}")

    try:
        # Import the module
        module = importlib.import_module(module_name)
        print(f"✅ Successfully imported {module_name}")

        # Check for .so module
        so_module_name = f"{module_name}.{module_name.split('.')[-1]}"
        try:
            so_module = importlib.import_module(so_module_name)
            print(f"\n📦 .so module found: {so_module_name}")
            print(f"   Exported symbols: {', '.join(dir(so_module))}")

            if hasattr(so_module, '__all__'):
                print(f"   __all__ list: {so_module.__all__}")
        except ImportError:
            print(f"⚠️  No .so submodule at {so_module_name}")

        # List all exported symbols
        all_exports = dir(module)
        print(f"\n📋 All exports from {module_name}:")
        for item in sorted(all_exports):
            if not item.startswith('_'):
                print(f"   - {item}")

        # Check for expected classes
        if expected_classes:
            print(f"\n🔍 Checking for expected classes:")
            for cls in expected_classes:
                if hasattr(module, cls):
                    print(f"   ✅ {cls} found")
                else:
                    print(f"   ❌ {cls} NOT FOUND")

                    # Try direct import
                    try:
                        exec(f"from {module_name} import {cls}")
                        print(f"      (But direct import works!)")
                    except ImportError as e:
                        print(f"      (Direct import also fails: {e})")

        return True

    except ImportError as e:
        print(f"❌ Failed to import {module_name}: {e}")
        return False


def check_build_artifacts():
    """Check for compiled build artifacts."""
    print_header("Checking Build Artifacts")

    # Check for target/release and target/debug
    target_dir = Path("target")

    if not target_dir.exists():
        print("❌ No target/ directory found. Have you run maturin build?")
        return False

    print("✅ target/ directory exists")

    # Look for .so files
    so_files = list(target_dir.rglob("*.so"))
    if so_files:
        print(f"\n📦 Found {len(so_files)} .so file(s):")
        for so_file in so_files:
            print(f"   - {so_file}")
    else:
        print("⚠️  No .so files found in target/")

    # Look for .whl files
    whl_files = list(target_dir.rglob("*.whl"))
    if whl_files:
        print(f"\n📦 Found {len(whl_files)} .whl file(s):")
        for whl_file in whl_files:
            print(f"   - {whl_file}")
    else:
        print("⚠️  No .whl files found in target/")

    return True


def run_cargo_clean():
    """Run cargo clean for the package."""
    print_header("Running cargo clean")

    try:
        result = subprocess.run(
            ["cargo", "clean"],
            capture_output=True,
            text=True,
            check=True
        )
        print("✅ cargo clean completed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ cargo clean failed: {e}")
        print(f"   stderr: {e.stderr}")
        return False
    except FileNotFoundError:
        print("❌ cargo command not found. Is Rust installed?")
        return False


def suggest_rebuild_steps(package_name=None):
    """Suggest steps to rebuild with maturin."""
    print_header("Suggested Rebuild Steps")

    print("\n🔧 For the known caching issue with maturin develop:")
    print("   See: https://github.com/PyO3/maturin/issues/381")
    print()
    print("   Try this rebuild sequence:")
    if package_name:
        print(f"   1. cargo clean -p {package_name}")
    else:
        print("   1. cargo clean")
    print("   2. uv run --with maturin --with patchelf maturin build --release")
    print("   3. uv pip install target/wheels/*.whl --force-reinstall")
    print()
    print("🔧 Or for development (faster but may have caching issues):")
    print("   1. cargo clean")
    print("   2. uv run --with maturin maturin develop")
    print()
    print("⚠️  Note: If new methods still don't appear after these steps,")
    print("   verify that the class is registered in the #[pymodule] function.")


def main():
    """Main diagnostic workflow."""
    print("🔍 Maturin + PyO3 Diagnostic Tool")
    print("=" * 60)

    if len(sys.argv) < 2:
        print("\nUsage: python diagnose_maturin.py <module_name> [expected_class1] [expected_class2] ...")
        print("\nExample:")
        print("  python diagnose_maturin.py pubmed_client Client PubMedClient SearchQuery")
        print("\nOr run without arguments in a package directory:")
        print("  python diagnose_maturin.py")
        sys.exit(1)

    module_name = sys.argv[1]
    expected_classes = sys.argv[2:] if len(sys.argv) > 2 else None

    # Run diagnostics
    check_build_artifacts()
    check_module_exports(module_name, expected_classes)

    # Suggest rebuild steps
    suggest_rebuild_steps()


if __name__ == "__main__":
    main()
