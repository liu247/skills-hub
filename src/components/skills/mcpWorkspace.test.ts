import { describe, expect, it } from 'vitest'
import { groupMcpServersBySource } from './mcpWorkspace'
import type { McpServerDto } from './types'

const server = (overrides: Partial<McpServerDto>): McpServerDto => ({
  id: 'server-1',
  name: 'example',
  transport: 'stdio',
  command: 'npx',
  args: ['example-server'],
  env: {},
  cwd: null,
  url: null,
  headers: {},
  enabled: true,
  proxy_enabled: true,
  source_url: null,
  source_path: null,
  secret_refs: [],
  targets: [],
  ...overrides,
})

describe('groupMcpServersBySource', () => {
  it('groups imported servers by repository and keeps manual configuration together', () => {
    const collections = groupMcpServersBySource([
      server({ id: 'github-1', source_url: 'https://github.com/acme/mcp-tools', source_path: 'mcp.json' }),
      server({ id: 'github-2', source_url: 'https://github.com/acme/mcp-tools', source_path: '.mcp.json' }),
      server({ id: 'manual-1', name: 'local-tools' }),
    ])

    expect(collections).toMatchObject([
      { key: 'https://github.com/acme/mcp-tools', label: 'acme/mcp-tools', serviceCount: 2 },
      { key: 'manual', label: 'Manual configuration', serviceCount: 1 },
    ])
  })
})
