#!/usr/bin/env python3
"""
Inspect the pubmed_client module and extract type information.

This script uses runtime introspection to generate a simple stub file
for the pubmed_client Python module.
"""

import inspect
import sys
from typing import Any


def get_signature_str(obj: Any) -> str:
    """Extract signature string from an object if available."""
    try:
        sig = inspect.signature(obj)
        return str(sig)
    except (ValueError, TypeError):
        # Check for __text_signature__ (PyO3 provides this)
        if hasattr(obj, "__text_signature__"):
            return obj.__text_signature__
        return "()"


def inspect_class(cls: type, indent: str = "") -> list[str]:
    """Inspect a class and generate stub lines."""
    lines = []
    lines.append(f"{indent}class {cls.__name__}:")

    # Add docstring if available
    if cls.__doc__:
        doc = cls.__doc__.strip().split("\n")[0]  # First line only
        lines.append(f'{indent}    """{doc}"""')

    # Inspect methods
    methods_found = False
    for name in dir(cls):
        if name.startswith("_") and name not in ["__init__", "__repr__", "__str__", "__len__"]:
            continue

        try:
            attr = getattr(cls, name)
            if callable(attr):
                sig = get_signature_str(attr)
                lines.append(f"{indent}    def {name}{sig}: ...")
                methods_found = True
        except AttributeError:
            pass

    if not methods_found:
        lines.append(f"{indent}    pass")

    return lines


def generate_stubs(module_name: str) -> str:
    """Generate stub file content for a module."""
    try:
        module = __import__(module_name)
    except ImportError as e:
        print(f"Error: Cannot import module '{module_name}': {e}", file=sys.stderr)
        sys.exit(1)

    lines = [
        f'"""Type stubs for {module_name} (auto-generated)"""',
        "",
    ]

    # Get version if available
    if hasattr(module, "__version__"):
        lines.append(f"__version__: str")
        lines.append("")

    # Inspect all classes and functions in the module
    classes = []
    functions = []

    for name in dir(module):
        if name.startswith("_"):
            continue

        try:
            obj = getattr(module, name)
            if inspect.isclass(obj):
                classes.append((name, obj))
            elif callable(obj):
                functions.append((name, obj))
        except AttributeError:
            pass

    # Generate function stubs
    for name, func in functions:
        sig = get_signature_str(func)
        lines.append(f"def {name}{sig}: ...")
        lines.append("")

    # Generate class stubs
    for name, cls in classes:
        class_lines = inspect_class(cls)
        lines.extend(class_lines)
        lines.append("")

    # Generate __all__
    all_names = [name for name, _ in classes] + [name for name, _ in functions]
    if hasattr(module, "__version__"):
        all_names.insert(0, "__version__")

    lines.append("__all__ = [")
    for name in sorted(all_names):
        lines.append(f'    "{name}",')
    lines.append("]")

    return "\n".join(lines)


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: python inspect_module.py <module_name> [output_file]")
        print("Example: python inspect_module.py pubmed_client pubmed_client_auto.pyi")
        sys.exit(1)

    module_name = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None

    stub_content = generate_stubs(module_name)

    if output_file:
        with open(output_file, "w") as f:
            f.write(stub_content)
        print(f"✓ Generated stub file: {output_file}")
    else:
        print(stub_content)


if __name__ == "__main__":
    main()
