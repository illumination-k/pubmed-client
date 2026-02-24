import { describe, expect, it } from 'vitest'
import { SearchQuery } from '../index.js'

describe('SearchQuery', () => {
  describe('Basic Methods', () => {
    it('should create a new SearchQuery', () => {
      const query = new SearchQuery()
      expect(query).toBeDefined()
    })

    it('should add search terms with query()', () => {
      const query = new SearchQuery()
        .query('covid-19')
        .query('treatment')

      const result = query.build()
      expect(result).toBe('covid-19 treatment')
    })

    it('should add multiple search terms with terms()', () => {
      const query = new SearchQuery()
        .terms(['covid-19', 'vaccine', 'efficacy'])

      const result = query.build()
      expect(result).toBe('covid-19 vaccine efficacy')
    })

    it('should filter empty strings in query()', () => {
      const query = new SearchQuery()
        .query('covid-19')
        .query('')
        .query('  ')
        .query('treatment')

      const result = query.build()
      expect(result).toBe('covid-19 treatment')
    })

    it('should filter empty strings in terms()', () => {
      const query = new SearchQuery()
        .terms(['covid-19', '', '  ', 'vaccine'])

      const result = query.build()
      expect(result).toBe('covid-19 vaccine')
    })

    it('should throw error when building empty query', () => {
      const query = new SearchQuery()
      expect(() => query.build()).toThrow()
    })

    it('should set and get limit', () => {
      const query = new SearchQuery()
        .query('cancer')
        .setLimit(100)

      expect(query.limit).toBe(100)
    })

    it('should have default limit of 20', () => {
      const query = new SearchQuery().query('cancer')
      expect(query.limit).toBe(20)
    })

    it('should clamp limit to minimum of 1', () => {
      const query = new SearchQuery().query('cancer').setLimit(0)
      expect(query.limit).toBe(1)
    })

    it('should clamp limit to maximum of 10000', () => {
      const query = new SearchQuery().query('cancer').setLimit(20000)
      expect(query.limit).toBe(10000)
    })
  })

  describe('Date Filtering Methods', () => {
    it('should filter by published in year', () => {
      const query = new SearchQuery()
        .query('covid-19')
        .publishedInYear(2024)

      const result = query.build()
      expect(result).toContain('2024[pdat]')
    })

    it('should filter by published between years', () => {
      const query = new SearchQuery()
        .query('cancer')
        .publishedBetween(2020, 2024)

      const result = query.build()
      expect(result).toContain('2020:2024[pdat]')
    })

    it('should filter by published after year', () => {
      const query = new SearchQuery()
        .query('crispr')
        .publishedAfter(2020)

      const result = query.build()
      expect(result).toContain('2020:3000[pdat]')
    })

    it('should filter by published before year', () => {
      const query = new SearchQuery()
        .query('genome')
        .publishedBefore(2020)

      const result = query.build()
      expect(result).toContain('1900:2020[pdat]')
    })

    it('should throw error for invalid year', () => {
      const query = new SearchQuery().query('cancer')
      expect(() => query.publishedInYear(1700)).toThrow()
      expect(() => query.publishedInYear(3500)).toThrow()
    })

    it('should throw error when start year > end year', () => {
      const query = new SearchQuery().query('cancer')
      expect(() => query.publishedBetween(2024, 2020)).toThrow()
    })
  })

  describe('Article Type and Language Filtering', () => {
    it('should filter by article type', () => {
      const query = new SearchQuery()
        .query('cancer')
        .articleType('Clinical Trial')

      const result = query.build()
      expect(result).toContain('Clinical Trial[pt]')
    })

    it('should filter by article type case-insensitively', () => {
      const query = new SearchQuery()
        .query('cancer')
        .articleType('clinical trial')

      const result = query.build()
      expect(result).toContain('Clinical Trial[pt]')
    })

    it('should filter by RCT shorthand', () => {
      const query = new SearchQuery()
        .query('treatment')
        .articleType('RCT')

      const result = query.build()
      expect(result).toContain('Randomized Controlled Trial[pt]')
    })

    it('should filter by multiple article types', () => {
      const query = new SearchQuery()
        .query('treatment')
        .articleTypes(['RCT', 'Meta-Analysis'])

      const result = query.build()
      expect(result).toContain('Randomized Controlled Trial[pt]')
      expect(result).toContain('Meta-Analysis[pt]')
    })

    it('should throw error for invalid article type', () => {
      const query = new SearchQuery().query('cancer')
      expect(() => query.articleType('invalid type')).toThrow()
    })

    it('should filter by language', () => {
      const query = new SearchQuery()
        .query('cancer')
        .language('English')

      const result = query.build()
      expect(result).toContain('English[la]')
    })

    it('should filter by language case-insensitively', () => {
      const query = new SearchQuery()
        .query('cancer')
        .language('japanese')

      const result = query.build()
      expect(result).toContain('Japanese[la]')
    })

    it('should handle custom language', () => {
      const query = new SearchQuery()
        .query('research')
        .language('Esperanto')

      const result = query.build()
      expect(result).toContain('Esperanto[la]')
    })
  })

  describe('Open Access Filtering', () => {
    it('should filter by free full text', () => {
      const query = new SearchQuery()
        .query('cancer')
        .freeFullTextOnly()

      const result = query.build()
      expect(result).toContain('free full text[sb]')
    })

    it('should filter by full text', () => {
      const query = new SearchQuery()
        .query('diabetes')
        .fullTextOnly()

      const result = query.build()
      expect(result).toContain('full text[sb]')
    })

    it('should filter by PMC only', () => {
      const query = new SearchQuery()
        .query('genomics')
        .pmcOnly()

      const result = query.build()
      expect(result).toContain('pubmed pmc[sb]')
    })

    it('should filter by has abstract', () => {
      const query = new SearchQuery()
        .query('genetics')
        .hasAbstract()

      const result = query.build()
      expect(result).toContain('hasabstract')
    })
  })

  describe('Field-Specific Search', () => {
    it('should search in title', () => {
      const query = new SearchQuery()
        .titleContains('machine learning')

      const result = query.build()
      expect(result).toContain('machine learning[ti]')
    })

    it('should search in abstract', () => {
      const query = new SearchQuery()
        .abstractContains('neural networks')

      const result = query.build()
      expect(result).toContain('neural networks[tiab]')
    })

    it('should search in title or abstract', () => {
      const query = new SearchQuery()
        .titleOrAbstract('CRISPR')

      const result = query.build()
      expect(result).toContain('CRISPR[tiab]')
    })

    it('should filter by journal', () => {
      const query = new SearchQuery()
        .query('cancer')
        .journal('Nature')

      const result = query.build()
      expect(result).toContain('Nature[ta]')
    })

    it('should filter by grant number', () => {
      const query = new SearchQuery()
        .grantNumber('R01AI123456')

      const result = query.build()
      expect(result).toContain('R01AI123456[gr]')
    })
  })

  describe('Advanced Search Methods', () => {
    it('should filter by MeSH term', () => {
      const query = new SearchQuery()
        .meshTerm('Neoplasms')

      const result = query.build()
      expect(result).toContain('Neoplasms[mh]')
    })

    it('should filter by MeSH major topic', () => {
      const query = new SearchQuery()
        .meshMajorTopic('Diabetes Mellitus')

      const result = query.build()
      expect(result).toContain('Diabetes Mellitus[majr]')
    })

    it('should filter by multiple MeSH terms', () => {
      const query = new SearchQuery()
        .meshTerms(['Neoplasms', 'Antineoplastic Agents'])

      const result = query.build()
      expect(result).toContain('Neoplasms[mh]')
      expect(result).toContain('Antineoplastic Agents[mh]')
    })

    it('should filter by MeSH subheading', () => {
      const query = new SearchQuery()
        .meshTerm('Diabetes Mellitus')
        .meshSubheading('drug therapy')

      const result = query.build()
      expect(result).toContain('Diabetes Mellitus[mh]')
      expect(result).toContain('drug therapy[sh]')
    })

    it('should filter by author', () => {
      const query = new SearchQuery()
        .query('machine learning')
        .author('Williams K')

      const result = query.build()
      expect(result).toContain('Williams K[au]')
    })

    it('should filter by first author', () => {
      const query = new SearchQuery()
        .query('cancer')
        .firstAuthor('Smith J')

      const result = query.build()
      expect(result).toContain('Smith J[1au]')
    })

    it('should filter by last author', () => {
      const query = new SearchQuery()
        .query('genomics')
        .lastAuthor('Johnson M')

      const result = query.build()
      expect(result).toContain('Johnson M[lastau]')
    })

    it('should filter by affiliation', () => {
      const query = new SearchQuery()
        .query('cardiology')
        .affiliation('Harvard Medical School')

      const result = query.build()
      expect(result).toContain('Harvard Medical School[ad]')
    })

    it('should filter by ORCID', () => {
      const query = new SearchQuery()
        .orcid('0000-0001-2345-6789')

      const result = query.build()
      expect(result).toContain('0000-0001-2345-6789[auid]')
    })

    it('should filter by human studies only', () => {
      const query = new SearchQuery()
        .query('drug treatment')
        .humanStudiesOnly()

      const result = query.build()
      expect(result).toContain('humans[mh]')
    })

    it('should filter by animal studies only', () => {
      const query = new SearchQuery()
        .query('preclinical research')
        .animalStudiesOnly()

      const result = query.build()
      expect(result).toContain('animals[mh]')
    })

    it('should filter by age group', () => {
      const query = new SearchQuery()
        .query('pediatric medicine')
        .ageGroup('Child')

      const result = query.build()
      expect(result).toContain('Child[mh]')
    })

    it('should add custom filter', () => {
      const query = new SearchQuery()
        .query('research')
        .customFilter('humans[mh]')

      const result = query.build()
      expect(result).toContain('humans[mh]')
    })
  })

  describe('Boolean Logic Methods', () => {
    it('should combine queries with AND', () => {
      const q1 = new SearchQuery().query('covid-19')
      const q2 = new SearchQuery().query('vaccine')
      const combined = q1.and(q2)

      const result = combined.build()
      expect(result).toBe('(covid-19) AND (vaccine)')
    })

    it('should combine queries with OR', () => {
      const q1 = new SearchQuery().query('diabetes')
      const q2 = new SearchQuery().query('hypertension')
      const combined = q1.or(q2)

      const result = combined.build()
      expect(result).toBe('(diabetes) OR (hypertension)')
    })

    it('should negate a query', () => {
      const query = new SearchQuery().query('cancer').negate()

      const result = query.build()
      expect(result).toBe('NOT (cancer)')
    })

    it('should exclude a query', () => {
      const base = new SearchQuery().query('cancer treatment')
      const exclude = new SearchQuery().query('animal studies')
      const filtered = base.exclude(exclude)

      const result = filtered.build()
      expect(result).toBe('(cancer treatment) NOT (animal studies)')
    })

    it('should group a query', () => {
      const query = new SearchQuery().query('cancer').group()

      const result = query.build()
      expect(result).toBe('(cancer)')
    })

    it('should handle complex boolean chaining', () => {
      const q1 = new SearchQuery().query('machine learning')
      const q2 = new SearchQuery().query('medicine')
      const q3 = new SearchQuery().query('veterinary')

      const combined = q1.and(q2).exclude(q3)

      const result = combined.build()
      expect(result).toBe('((machine learning) AND (medicine)) NOT (veterinary)')
    })
  })

  describe('Complex Query Building', () => {
    it('should build complex query with multiple filters', () => {
      const query = new SearchQuery()
        .query('covid-19 treatment')
        .titleContains('immunotherapy')
        .journal('Nature')
        .freeFullTextOnly()
        .articleType('Clinical Trial')
        .language('English')
        .publishedBetween(2020, 2024)

      const result = query.build()
      expect(result).toContain('covid-19 treatment')
      expect(result).toContain('immunotherapy[ti]')
      expect(result).toContain('Nature[ta]')
      expect(result).toContain('free full text[sb]')
      expect(result).toContain('Clinical Trial[pt]')
      expect(result).toContain('English[la]')
      expect(result).toContain('2020:2024[pdat]')
    })

    it('should build query with MeSH and author filters', () => {
      const query = new SearchQuery()
        .meshTerm('Neoplasms')
        .author('Smith J')
        .humanStudiesOnly()
        .affiliation('Harvard')

      const result = query.build()
      expect(result).toContain('Neoplasms[mh]')
      expect(result).toContain('Smith J[au]')
      expect(result).toContain('humans[mh]')
      expect(result).toContain('Harvard[ad]')
    })
  })

})
