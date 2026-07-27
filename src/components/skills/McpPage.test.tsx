import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import McpPage from './McpPage'

const t = ((key: string, options?: Record<string, unknown>) => ({
  'mcp.title': 'MCP',
  'mcp.help': 'Manage shared MCP services.',
  'mcp.add': 'Add MCP',
  'mcp.managed': 'Managed services',
  'mcp.apps': 'Synced apps',
  'mcp.credentials': 'Credentials ready',
  'mcp.syncStatus': 'Sync status',
  'mcp.allNormal': 'All healthy',
  'mcp.sourceCount': `${options?.count ?? 0} MCP services`,
  'mcp.manualSource': 'Manual configuration',
  'mcp.localSource': `Local import · ${options?.app ?? ''}`,
  'mcp.sourceSearch': 'Search MCP services…',
  'mcp.allSources': 'All sources',
  'mcp.sortUpdated': 'Recently updated',
  'mcp.batchSync': 'Sync all sources',
  'mcp.scanLocal': 'Scan local MCP',
  'mcp.gridView': 'Grid view',
  'mcp.listView': 'List view',
  'mcp.back': 'All sources',
  'mcp.sync': 'Sync all apps',
  delete: 'Delete',
  cancel: 'Cancel',
  save: 'Save',
  'mcp.name': 'Name',
  'mcp.transport': 'Transport',
  'mcp.command': 'Command',
  'mcp.args': 'Arguments',
  'mcp.url': 'Server URL',
  'mcp.headerName': 'Credential header name',
  'mcp.secrets': 'Credential names',
  'mcp.secretStatus': 'Credentials saved',
  'mcp.noSecrets': 'No credentials required',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('McpPage', () => {
  it('lands on source collections instead of raw MCP commands', () => {
    const markup = renderToStaticMarkup(
      <McpPage
        busy={false}
        initialManualEditor={false}
        servers={[
          { id: 'one', name: 'filesystem', transport: 'stdio', command: 'npx', args: ['example-server'], env: {}, cwd: null, url: null, headers: {}, enabled: true, proxy_enabled: true, source_url: 'https://github.com/acme/mcp-tools', source_path: 'mcp.json', secret_refs: [], targets: [] },
          { id: 'two', name: 'github', transport: 'http', command: null, args: [], env: {}, cwd: null, url: 'https://mcp.example.com', headers: {}, enabled: true, proxy_enabled: true, source_url: 'https://github.com/acme/mcp-tools', source_path: '.mcp.json', secret_refs: [], targets: [] },
          { id: 'three', name: 'local', transport: 'stdio', command: 'node', args: ['local.js'], env: {}, cwd: null, url: null, headers: {}, enabled: true, proxy_enabled: true, source_url: null, source_path: null, secret_refs: [], targets: [] },
        ]}
        onSave={async () => null}
        onSetSecret={async () => undefined}
        onDelete={async () => undefined}
        onSync={async () => undefined}
        onSetTargets={async () => undefined}
        onScanLocal={() => undefined}
        onOpenImport={() => undefined}
        onCloseManualEditor={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('acme/mcp-tools')
    expect(markup).toContain('2 MCP services')
    expect(markup).toContain('Manual configuration')
    expect(markup).not.toContain('npx example-server')
  })

  it('opens the manual editor immediately when entered from manual configuration', () => {
    const markup = renderToStaticMarkup(
      <McpPage
        busy={false}
        initialManualEditor
        servers={[]}
        onSave={async () => null}
        onSetSecret={async () => undefined}
        onDelete={async () => undefined}
        onSync={async () => undefined}
        onSetTargets={async () => undefined}
        onScanLocal={() => undefined}
        onOpenImport={() => undefined}
        onCloseManualEditor={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('Name')
    expect(markup).toContain('Credential names')
  })

  it('renders local discovery sources as readable App labels', () => {
    const markup = renderToStaticMarkup(<McpPage busy={false} initialManualEditor={false} servers={[{ id: 'local', name: 'filesystem', transport: 'stdio', command: 'npx', args: [], env: {}, cwd: null, url: null, headers: {}, enabled: true, proxy_enabled: true, source_url: 'local://codex', source_path: null, secret_refs: [], targets: [{ tool: 'codex', status: 'ok', last_error: null, synced_at: 1 }] }]} onSave={async () => null} onSetSecret={async () => undefined} onDelete={async () => undefined} onSync={async () => undefined} onSetTargets={async () => undefined} onScanLocal={() => undefined} onOpenImport={() => undefined} onCloseManualEditor={() => undefined} t={t} />)

    expect(markup).toContain('Local import · Codex')
  })
})
