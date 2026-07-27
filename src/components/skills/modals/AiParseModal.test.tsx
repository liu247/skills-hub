import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import AiParseModal from './AiParseModal'

const t = ((key: string) => ({
  'aiParse.title': 'AI parse source',
  'aiParse.sourceUrl': 'Source URL',
  'aiParse.provider': 'AI provider',
  'aiParse.parse': 'Parse with AI',
  'aiParse.confirm': 'Confirm configuration',
  'aiParse.cancel': 'Cancel',
  'aiParse.noProvider': 'Configure an enabled AI provider first.',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('AiParseModal', () => {
  it('does not render a confirmation action before AI returns a plan', () => {
    const markup = renderToStaticMarkup(
      <AiParseModal
        open
        mode="mcp"
        busy={false}
        providers={[{ provider: 'openai', enabled: true, model: 'gpt-4.1-mini', base_url: 'https://api.openai.com/v1', has_api_key: true }]}
        onClose={() => undefined}
        onParse={async () => null}
        onConfirm={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('Parse with AI')
    expect(markup).not.toContain('Confirm configuration')
  })
})
