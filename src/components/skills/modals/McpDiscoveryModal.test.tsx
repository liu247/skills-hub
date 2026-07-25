import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import McpDiscoveryModal from './McpDiscoveryModal'

const t = ((key: string, options?: Record<string, unknown>) => ({
  'mcp.localReviewTitle': 'Review local MCP services',
  'mcp.localReviewSummary': `Scanned ${options?.hosts ?? 0} apps and found ${options?.count ?? 0} services.`,
  'mcp.localCredentialMigration': 'Selected literal credentials will move to your OS credential store.',
  'mcp.adoptSelected': `Manage selected (${options?.count ?? 0})`,
  conflict: 'Conflict', consistent: 'Consistent', close: 'Close',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('McpDiscoveryModal', () => {
  it('shows the selected-service credential migration notice without exposing secret values', () => {
    const markup = renderToStaticMarkup(<McpDiscoveryModal open loading={false} onClose={() => undefined} onImport={() => undefined} t={t} plan={{ total_hosts_scanned: 2, total_servers_found: 1, groups: [{ name: 'private', has_conflict: false, variants: [{ host: 'codex', path: '~/.codex/config.toml', name: 'private', transport: 'stdio', command: 'npx', args: [], url: null, env: { API_TOKEN: '${API_TOKEN}' }, headers: {}, credential_names: ['API_TOKEN'] }] }] }} />)

    expect(markup).toContain('Selected literal credentials will move to your OS credential store.')
    expect(markup).toContain('API_TOKEN')
    expect(markup).not.toContain('local-secret')
  })
})
