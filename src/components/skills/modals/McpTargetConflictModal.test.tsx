import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import McpTargetConflictModal from './McpTargetConflictModal'

const t = ((key: string) => ({
  'mcp.targetConflictTitle': 'Existing App configuration',
  'mcp.targetConflictBody': 'Kiro already has a standalone configuration for playwright.',
  'mcp.keepLocal': 'Keep local configuration',
  'mcp.replaceAndManage': 'Replace and manage',
  'tools.kiro': 'Kiro',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('McpTargetConflictModal', () => {
  it('explains the conflict and requires an explicit replacement action', () => {
    const markup = renderToStaticMarkup(
      <McpTargetConflictModal
        open
        busy={false}
        tools={['kiro']}
        serverName="playwright"
        onCancel={() => undefined}
        onReplace={() => undefined}
        t={t}
      />,
    )
    expect(markup).toContain('Existing App configuration')
    expect(markup).toContain('Kiro')
    expect(markup).toContain('Keep local configuration')
    expect(markup).toContain('Replace and manage')
  })
})
