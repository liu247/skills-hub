import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import AiProviderSettings from './AiProviderSettings'

const t = ((key: string) => ({
  'aiSettings.title': 'AI parser configuration',
  'aiSettings.description': 'Configure the AI providers used to analyse Skills and MCP sources.',
  'aiSettings.configured': 'API key configured',
  'aiSettings.notConfigured': 'API key not configured',
  'aiSettings.enable': 'Enable provider',
  'aiSettings.model': 'Model',
  'aiSettings.baseUrl': 'Base URL',
  'aiSettings.apiKey': 'API key',
  'aiSettings.save': 'Save provider',
  'aiSettings.clear': 'Clear key',
  'aiSettings.test': 'Test connection',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('AiProviderSettings', () => {
  it('shows credential state without rendering the saved API key', () => {
    const markup = renderToStaticMarkup(
      <AiProviderSettings
        providers={[{ provider: 'openai', enabled: true, model: 'gpt-4.1-mini', base_url: 'https://api.openai.com/v1', has_api_key: true }]}
        onSave={() => undefined}
        onSetApiKey={() => undefined}
        onDeleteApiKey={() => undefined}
        onTest={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('API key configured')
    expect(markup).not.toContain('secret-value')
    expect(markup).toContain('type="password"')
    expect(markup).toContain('Test connection')
  })
})
