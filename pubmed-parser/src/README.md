# Parser XML Strategy

`pubmed-parser` uses XML parsing strategies based on the shape of the data being read.
New parser code should choose the first strategy that fits:

1. Serde deserialization (`quick_xml::de::from_str`) is for top-level, structurally regular XML where the XML shape maps cleanly to Rust types.
2. Reader event parsing (`quick_xml::Reader`) is for nested XML, mixed content, repeated elements, attributes, or any field where malformed input should be visible as an XML parsing error.
3. String `find()` and slicing helpers are legacy compatibility utilities. Do not use them for new XML structure extraction.

PMC metadata and section parsing should prefer the Reader helpers in `pmc/parser/reader_utils.rs`.
Those helpers centralize reader configuration and common operations such as attribute lookup, text collection, and nested element skipping.

The legacy helpers in `common/xml_utils.rs` remain for compatibility with older code and tests, but XML structure helpers such as `extract_text_between`, `find_all_tags`, and `extract_element_content` are deprecated.
They can silently miss malformed XML, match tag-like text inside comments or character data, and depend on exact attribute order or whitespace.

Pure text post-processing helpers such as entity decoding or stripping already-extracted inline tags are not part of that legacy strategy by themselves.
They are acceptable after XML structure has been selected by serde or Reader parsing.
