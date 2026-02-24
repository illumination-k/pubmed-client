import { describe, expect, it } from 'vitest'
import { WasmClientConfig, WasmPubMedClient } from '../pkg/pubmed_client_wasm.js'

describe('WASM Client Basic Tests', () => {
  describe('WasmClientConfig', () => {
    it('should create config successfully', () => {
      const config = new WasmClientConfig()
      expect(config).toBeDefined()
      config.free()
    })

    it('should handle invalid property gracefully', () => {
      const config = new WasmClientConfig()
      // TypeScript will catch this, but test runtime behavior
      expect(() => {
        ;(config as any).invalid_property = 'test'
      }).not.toThrow()
      config.free()
    })
  })

  describe('WasmPubMedClient Creation', () => {
    it('should create client successfully', () => {
      const client = new WasmPubMedClient()
      expect(client).toBeDefined()
      expect(typeof client.search_articles).toBe('function')
      client.free()
    })

    it('should create test client successfully', () => {
      const client = WasmPubMedClient.new_for_testing()
      expect(client).toBeDefined()
      expect(typeof client.search_articles).toBe('function')
      client.free()
    })

    it('should create client with config successfully', () => {
      const config = new WasmClientConfig()
      config.email = 'test@example.com'

      const client = WasmPubMedClient.with_config(config)
      expect(client).toBeDefined()

      client.free()
      // Don't free config after it's been used with a client
      // config.free()
    })
  })

  describe('Convert to Markdown', () => {
    it('should fail with invalid full text object', () => {
      const client = WasmPubMedClient.new_for_testing()

      expect(() => {
        client.convert_to_markdown(null as any)
      }).toThrow()

      client.free()
    })
  })
})
