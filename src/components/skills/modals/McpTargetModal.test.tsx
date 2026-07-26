import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import McpTargetModal from './McpTargetModal'

const t = ((key: string) => ({
  'mcp.targetsTitle': 'Manage sync targets',
  'mcp.targetsHelp': 'Choose the Apps that should receive this MCP service.',
  'mcp.saveTargets': 'Save targets',
  cancel: 'Cancel', 'tools.codex': 'Codex', 'tools.claude_code': 'Claude Code', 'tools.claude_3p': 'Claude-3p', 'tools.kiro': 'Kiro', 'tools.reasonix': 'Reasonix',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('McpTargetModal', () => {
  it('renders all supported Apps and marks persisted targets as selected', () => {
    const markup = renderToStaticMarkup(<McpTargetModal open busy={false} selectedTools={['codex', 'kiro']} onClose={() => undefined} onSave={() => undefined} t={t} />)
    expect(markup).toContain('Manage sync targets')
    expect(markup).toContain('Codex')
    expect(markup).toContain('Claude Code')
    expect(markup).toContain('Claude-3p')
    expect(markup).toContain('Kiro')
    expect(markup).toContain('checked')
  })
})
