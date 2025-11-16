# PubMed Search Field Tags - Official Reference

## Official Documentation Sources

**PRIMARY RESOURCES** - Always verify field tags against these sources:

1. **PubMed Help - Search Field Tags**
   - URL: https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags
   - Status: Most authoritative source for field tag validation
   - Contains: Complete list of valid short-form tags with descriptions

2. **NCBI E-utilities Documentation**
   - URL: https://www.ncbi.nlm.nih.gov/books/NBK25499/
   - Status: Technical documentation for API implementation
   - Contains: Query syntax, field tag usage in API calls

## Validated Field Tags

These tags have been verified against official NCBI PubMed documentation and are guaranteed to work correctly.

### Article Content Fields

| Tag      | Description                             | Example           |
| -------- | --------------------------------------- | ----------------- |
| `[ti]`   | Title                                   | `cancer[ti]`      |
| `[tiab]` | Title/Abstract                          | `treatment[tiab]` |
| `[ab]`   | Abstract                                | `mechanism[ab]`   |
| `[tw]`   | Text Word (title, abstract, other text) | `therapy[tw]`     |

### Author Fields

| Tag        | Description               | Example                     |
| ---------- | ------------------------- | --------------------------- |
| `[au]`     | Author                    | `Smith J[au]`               |
| `[1au]`    | First Author              | `Smith J[1au]`              |
| `[lastau]` | Last Author               | `Johnson K[lastau]`         |
| `[ad]`     | Affiliation               | `Harvard[ad]`               |
| `[auid]`   | Author Identifier (ORCID) | `0000-0001-2345-6789[auid]` |

### Journal/Publication Fields

| Tag      | Description                | Example         |
| -------- | -------------------------- | --------------- |
| `[ta]`   | Journal Title Abbreviation | `Nature[ta]`    |
| `[jour]` | Journal                    | `Science[jour]` |
| `[is]`   | ISSN                       | `0028-0836[is]` |
| `[vi]`   | Volume                     | `123[vi]`       |
| `[ip]`   | Issue                      | `5[ip]`         |
| `[pg]`   | Pagination                 | `100-110[pg]`   |

### Subject/Classification Fields

| Tag      | Description      | Example          |
| -------- | ---------------- | ---------------- |
| `[mh]`   | MeSH Terms       | `Neoplasms[mh]`  |
| `[majr]` | MeSH Major Topic | `COVID-19[majr]` |
| `[sh]`   | MeSH Subheading  | `therapy[sh]`    |
| `[nm]`   | Substance Name   | `Aspirin[nm]`    |

### Date Fields

| Tag      | Description         | Example            |
| -------- | ------------------- | ------------------ |
| `[pdat]` | Publication Date    | `2023[pdat]`       |
| `[edat]` | Entry Date          | `2023/01/01[edat]` |
| `[mdat]` | Modification Date   | `2023/12/31[mdat]` |
| `[dp]`   | Date of Publication | `2023[dp]`         |
| `[dcom]` | Date Completed      | `2023[dcom]`       |
| `[mhda]` | MeSH Date           | `2023[mhda]`       |
| `[lr]`   | Last Revision Date  | `2023[lr]`         |

### Other Fields

| Tag    | Description              | Example                              |
| ------ | ------------------------ | ------------------------------------ |
| `[la]` | Language                 | `eng[la]`                            |
| `[pt]` | Publication Type         | `Review[pt]`                         |
| `[sb]` | Subset                   | `free full text[sb]`                 |
| `[gr]` | Grant Number             | `R01CA123456[gr]`                    |
| `[si]` | Secondary Source ID      | `ClinicalTrials.gov/NCT12345678[si]` |
| `[ps]` | Personal Name as Subject | `Einstein A[ps]`                     |

## Invalid/Non-Existent Tags

These tags **DO NOT EXIST** in PubMed and should never be used.

### Common Mistakes

| Invalid Tag  | Why It's Invalid        | Recommended Alternative                          |
| ------------ | ----------------------- | ------------------------------------------------ |
| `[Organism]` | Not a field tag         | Use MeSH terms with `[mh]`: `"Homo sapiens"[mh]` |
| `[organism]` | Not a field tag         | Use MeSH terms with `[mh]` or organism filter    |
| `[Title]`    | Long form not supported | Use short form: `[ti]`                           |
| `[Author]`   | Long form not supported | Use short form: `[au]`                           |
| `[Abstract]` | Not a standalone tag    | Use `[ab]` or `[tiab]`                           |
| `[keyword]`  | Not a field tag         | Use `[tw]` for text word search                  |
| `[subject]`  | Not a field tag         | Use `[mh]` for MeSH terms                        |

## Deprecated Tags

These tags still work but have preferred alternatives.

| Deprecated Tag | Status     | Recommended Alternative |
| -------------- | ---------- | ----------------------- |
| `[lang]`       | Deprecated | Use `[la]` instead      |

## Tag Syntax Rules

### Correct Syntax

```
✅ cancer[ti]              # Correct: lowercase tag in brackets
✅ "cancer treatment"[ti]  # Correct: quoted phrase with tag
✅ Smith J[au]             # Correct: author with tag
✅ 2023[pdat]              # Correct: year with date tag
```

### Incorrect Syntax

```
❌ cancer[Title]           # Wrong: long form not supported
❌ cancer[TI]              # Wrong: uppercase not standard
❌ cancer [ti]             # Wrong: space before bracket
❌ [ti]cancer              # Wrong: tag before term
```

## Search Query Examples

### Simple Field Search

```
COVID-19[ti]                          # Title contains "COVID-19"
Smith J[au]                           # Author is "Smith J"
Nature[ta]                            # Journal is "Nature"
2023[pdat]                            # Published in 2023
```

### Combined Field Searches

```
cancer[ti] AND therapy[tiab]          # Title has "cancer" AND title/abstract has "therapy"
Smith J[au] AND 2023[pdat]            # Author "Smith J" AND published in 2023
"machine learning"[tiab] AND Review[pt]  # Title/abstract has "machine learning" AND is a review
```

### Advanced Searches with MeSH

```
"Neoplasms"[mh]                       # MeSH term for neoplasms
"COVID-19"[majr]                      # Major topic is COVID-19
"Neoplasms/therapy"[mh]               # Neoplasms with therapy subheading
```

## Validation Checklist

When implementing or using field tags, verify:

- [ ] Tag uses **short form** (e.g., `[ti]` not `[Title]`)
- [ ] Tag is **lowercase** (standard convention)
- [ ] Tag appears **after** the search term (e.g., `cancer[ti]`)
- [ ] Tag is in the **validated list** above
- [ ] For unknown tags, **check official documentation** before use
- [ ] For organism searches, use **MeSH terms** instead of `[organism]`
- [ ] Deprecated tags replaced with **recommended alternatives**

## When to Re-validate

1. **Before implementing new field tags** - Check official docs first
2. **When queries fail unexpectedly** - Verify tag syntax and validity
3. **After NCBI updates** - Field tags may be added or deprecated
4. **When debugging SearchQuery** - Ensure tags are properly formatted

## Grep Patterns for Finding Tags in Code

```bash
# Find all field tag references in Rust code
grep -r '\[ti\]' pubmed-client/
grep -r '\[au\]' pubmed-client/
grep -r '\[mh\]' pubmed-client/

# Find potential field tag usage in filters
rg 'pub fn \w+.*&str.*\[' pubmed-client/src/pubmed/query/

# Scan for square bracket patterns (potential tags)
rg '\[[a-z]+\]' pubmed-client/src/
```

## Updating This Reference

When new field tags are validated or invalidated:

1. Update the appropriate table in this document
2. Update `CLAUDE.md` "Currently Validated Field Tags" section
3. Update `scripts/validate_field_tags.py` dictionaries
4. Run validation script to verify consistency
5. Update examples if needed

## Related Files

- **CLAUDE.md** - Project-level field tag guidelines
- **scripts/validate_field_tags.py** - Automated validation tool
- **pubmed-client/src/pubmed/query/filters.rs** - Filter implementation
- **pubmed-client/src/pubmed/query/advanced.rs** - Advanced search features
