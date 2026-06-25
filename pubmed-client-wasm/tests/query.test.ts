import { describe, expect, it } from 'vitest'
import { WasmSearchQuery } from '../pkg/pubmed_client_wasm.js'

// These tests exercise the WasmSearchQuery builder offline. `build()` is pure
// string construction with no network access, so the expected output mirrors the
// core `SearchQuery` builder field tags.

describe('WasmSearchQuery builder', () => {
  describe('search field filters', () => {
    it('builds title and title/abstract filters', () => {
      expect(new WasmSearchQuery().title('machine learning').build()).toBe('machine learning[ti]')
      expect(new WasmSearchQuery().title_abstract('CRISPR').build()).toBe('CRISPR[tiab]')
      expect(new WasmSearchQuery().abstract_contains('genome').build()).toBe('genome[tiab]')
    })

    it('builds author, affiliation and ORCID filters', () => {
      expect(new WasmSearchQuery().author('Williams K').build()).toBe('Williams K[au]')
      expect(new WasmSearchQuery().first_author('Smith J').build()).toBe('Smith J[1au]')
      expect(new WasmSearchQuery().last_author('Johnson M').build()).toBe('Johnson M[lastau]')
      expect(new WasmSearchQuery().affiliation('Harvard Medical School').build()).toBe(
        'Harvard Medical School[ad]'
      )
      expect(new WasmSearchQuery().orcid('0000-0001-2345-6789').build()).toBe(
        '0000-0001-2345-6789[auid]'
      )
    })

    it('builds MeSH filters', () => {
      expect(new WasmSearchQuery().mesh_term('Neoplasms').build()).toBe('Neoplasms[mh]')
      expect(new WasmSearchQuery().mesh_major_topic('Diabetes Mellitus, Type 2').build()).toBe(
        'Diabetes Mellitus, Type 2[majr]'
      )
      expect(new WasmSearchQuery().mesh_terms(['Neoplasms', 'Antineoplastic Agents']).build()).toBe(
        'Neoplasms[mh] AND Antineoplastic Agents[mh]'
      )
      expect(
        new WasmSearchQuery().mesh_term('Diabetes Mellitus').mesh_subheading('drug therapy').build()
      ).toBe('Diabetes Mellitus[mh] AND drug therapy[sh]')
    })

    it('builds identifier and metadata filters', () => {
      expect(new WasmSearchQuery().isbn('978-0123456789').build()).toBe('978-0123456789[ISBN]')
      expect(new WasmSearchQuery().issn('1234-5678').build()).toBe('1234-5678[ISSN]')
      expect(new WasmSearchQuery().grant_number('CA123456').build()).toBe('CA123456[gr]')
      expect(new WasmSearchQuery().has_abstract().build()).toBe('hasabstract')
    })

    it('builds study population filters', () => {
      expect(new WasmSearchQuery().humans_only().build()).toBe('humans[mh]')
      expect(new WasmSearchQuery().animal_studies_only().build()).toBe('animals[mh]')
      expect(new WasmSearchQuery().age_group('Child').build()).toBe('Child[mh]')
      expect(new WasmSearchQuery().organism_mesh('Mus musculus').build()).toBe('Mus musculus[mh]')
    })

    it('builds custom and multi-term filters', () => {
      expect(new WasmSearchQuery().custom_filter('custom[field]').build()).toBe('custom[field]')
      expect(new WasmSearchQuery().terms(['test', 'more']).build()).toBe('test more')
    })
  })

  describe('article types', () => {
    it('accepts a single type with legacy aliases', () => {
      const q = new WasmSearchQuery().article_type_str('clinical_trial')
      expect(q.build()).toContain('Clinical Trial[pt]')
    })

    it('accepts multiple types (OR logic)', () => {
      const q = new WasmSearchQuery().article_types_str(['Review', 'Meta-Analysis'])
      expect(q.build()).toBe('(Review[pt] OR Meta-Analysis[pt])')
    })

    it('treats an empty type list as a no-op', () => {
      expect(new WasmSearchQuery().query('cancer').article_types_str([]).build()).toBe('cancer')
    })

    it('throws on an unknown article type', () => {
      expect(() => new WasmSearchQuery().article_type_str('not-a-type')).toThrow()
    })
  })

  describe('date filters', () => {
    it('builds year and range filters', () => {
      expect(new WasmSearchQuery().published_in_year(2020).build()).toContain('2020')
      expect(new WasmSearchQuery().published_after(2018).build()).toContain('2018')
      expect(new WasmSearchQuery().published_before(2022).build()).toContain('2022')
      expect(new WasmSearchQuery().published_between(2019, 2021).build()).toContain('2019')
    })

    it('rejects out-of-range years', () => {
      expect(() => new WasmSearchQuery().published_in_year(100)).toThrow()
    })

    it('rejects an inverted range', () => {
      expect(() => new WasmSearchQuery().published_between(2022, 2020)).toThrow()
    })

    it('configures the result limit', () => {
      const q = new WasmSearchQuery().limit(25)
      expect(q.get_limit()).toBe(25)
    })
  })

  describe('boolean composition', () => {
    it('combines queries with AND / OR', () => {
      const a = new WasmSearchQuery().query('diabetes')
      const b = new WasmSearchQuery().query('hypertension')
      expect(a.or(b).build()).toBe('(diabetes) OR (hypertension)')

      const c = new WasmSearchQuery().query('cancer')
      const d = new WasmSearchQuery().query('treatment')
      expect(c.and(d).build()).toBe('(cancer) AND (treatment)')
    })

    it('negates and excludes queries', () => {
      expect(new WasmSearchQuery().query('cancer').negate().build()).toBe('NOT (cancer)')

      const base = new WasmSearchQuery().query('cancer treatment')
      const animal = new WasmSearchQuery().query('animal studies')
      expect(base.exclude(animal).build()).toBe('(cancer treatment) NOT (animal studies)')
    })

    it('groups a query in parentheses', () => {
      expect(new WasmSearchQuery().query('cancer').group().build()).toBe('(cancer)')
    })

    it('does not consume the operands of a boolean combination', () => {
      const a = new WasmSearchQuery().query('a')
      const b = new WasmSearchQuery().query('b')
      const combined = a.and(b)
      // a/b take &self, so they remain usable afterwards.
      expect(a.build()).toBe('a')
      expect(b.build()).toBe('b')
      expect(combined.build()).toBe('(a) AND (b)')
    })
  })
})
