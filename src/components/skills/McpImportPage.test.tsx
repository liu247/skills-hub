import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import McpImportPage from './McpImportPage'

const t = ((key: string) => ({
  'mcpImport.title': 'Add MCP',
  'mcpImport.github': 'GitHub',
  'mcpImport.manual': 'Manual configuration',
  'mcpImport.url': 'Repository or config URL',
  'mcpImport.scan': 'Scan configuration',
  'mcpImport.empty': 'Paste a GitHub repository or configuration URL to discover MCP servers.',
}[key] ?? key)) as unknown as TFunction

describe('McpImportPage', () => {
  it('provides GitHub discovery and manual configuration entry points', () => {
    const markup = renderToStaticMarkup(
      <McpImportPage busy={false} candidates={[]} onScan={() => undefined} onImport={() => undefined} onOpenManual={() => undefined} t={t} />,
    )
    expect(markup).toContain('Add MCP')
    expect(markup).toContain('Repository or config URL')
    expect(markup).toContain('Manual configuration')
  })
})
