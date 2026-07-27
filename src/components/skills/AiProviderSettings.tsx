import { memo, useEffect, useState } from 'react'
import { Bot, ChevronDown, KeyRound } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { AiProviderConfigDto } from './types'

type ProviderRowProps = {
  provider: AiProviderConfigDto
  onSave: (config: Omit<AiProviderConfigDto, 'has_api_key'>) => void
  onSetApiKey: (provider: AiProviderConfigDto['provider'], value: string) => void
  onDeleteApiKey: (provider: AiProviderConfigDto['provider']) => void
  onTest: (provider: AiProviderConfigDto['provider']) => void
  t: TFunction
}

const ProviderRow = ({ provider, onSave, onSetApiKey, onDeleteApiKey, onTest, t }: ProviderRowProps) => {
  const [enabled, setEnabled] = useState(provider.enabled)
  const [model, setModel] = useState(provider.model)
  const [baseUrl, setBaseUrl] = useState(provider.base_url)
  const [apiKey, setApiKey] = useState('')

  useEffect(() => {
    setEnabled(provider.enabled)
    setModel(provider.model)
    setBaseUrl(provider.base_url)
  }, [provider])

  const save = () => {
    onSave({ provider: provider.provider, enabled, model, base_url: baseUrl })
    if (apiKey.trim()) {
      onSetApiKey(provider.provider, apiKey)
      setApiKey('')
    }
  }

  return (
    <div className="settings-ai-provider">
      <div className="settings-item">
        <div className="settings-item-info">
          <div className="settings-item-title">{provider.provider}</div>
          <div className="settings-item-desc">
            <KeyRound size={13} aria-hidden="true" />
            {provider.has_api_key ? t('aiSettings.configured') : t('aiSettings.notConfigured')}
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle${enabled ? ' checked' : ''}`}
          aria-label={t('aiSettings.enable')}
          aria-pressed={enabled}
          onClick={() => setEnabled((current) => !current)}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-field">
        <label className="settings-label">{t('aiSettings.model')}</label>
        <input className="settings-input" value={model} onChange={(event) => setModel(event.target.value)} />
      </div>
      <div className="settings-field">
        <label className="settings-label">{t('aiSettings.baseUrl')}</label>
        <input className="settings-input mono" value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} />
      </div>
      <div className="settings-field">
        <label className="settings-label">{t('aiSettings.apiKey')}</label>
        <div className="settings-input-row">
          <input
            className="settings-input mono"
            type="password"
            value={apiKey}
            autoComplete="off"
            onChange={(event) => setApiKey(event.target.value)}
          />
          {provider.has_api_key ? <button type="button" className="btn btn-secondary settings-browse" onClick={() => onDeleteApiKey(provider.provider)}>{t('aiSettings.clear')}</button> : null}
        </div>
      </div>
      <div className="settings-ai-actions">
        <button type="button" className="btn btn-secondary btn-sm" disabled={!provider.has_api_key || !enabled} onClick={() => onTest(provider.provider)}>{t('aiSettings.test')}</button>
        <button type="button" className="btn btn-secondary btn-sm" onClick={save}>{t('aiSettings.save')}</button>
      </div>
    </div>
  )
}

type AiProviderSettingsProps = {
  providers: AiProviderConfigDto[]
  onSave: (config: Omit<AiProviderConfigDto, 'has_api_key'>) => void
  onSetApiKey: (provider: AiProviderConfigDto['provider'], value: string) => void
  onDeleteApiKey: (provider: AiProviderConfigDto['provider']) => void
  onTest: (provider: AiProviderConfigDto['provider']) => void
  t: TFunction
}

const AiProviderSettings = ({ providers, onSave, onSetApiKey, onDeleteApiKey, onTest, t }: AiProviderSettingsProps) => {
  const [selectedProviderId, setSelectedProviderId] = useState<AiProviderConfigDto['provider']>(providers[0]?.provider ?? 'openai')
  const selectedProvider = providers.find((provider) => provider.provider === selectedProviderId) ?? providers[0]

  return (
    <section className="settings-card">
      <div className="settings-card-head">
        <span className="settings-card-icon"><Bot size={18} /></span>
        <div><h2>{t('aiSettings.title')}</h2><p>{t('aiSettings.description')}</p></div>
      </div>
      <div className="settings-card-body settings-ai-provider-list">
        <div className="settings-field settings-ai-provider-selector">
          <label className="settings-label" htmlFor="ai-provider-select">{t('aiSettings.provider')}</label>
          <div className="settings-select-wrap">
            <select
              id="ai-provider-select"
              className="settings-select"
              value={selectedProvider?.provider ?? ''}
              onChange={(event) => setSelectedProviderId(event.target.value as AiProviderConfigDto['provider'])}
            >
              {providers.map((provider) => <option key={provider.provider} value={provider.provider}>{provider.provider}</option>)}
            </select>
            <ChevronDown className="settings-select-caret" aria-hidden="true" />
          </div>
        </div>
        {selectedProvider ? <ProviderRow provider={selectedProvider} onSave={onSave} onSetApiKey={onSetApiKey} onDeleteApiKey={onDeleteApiKey} onTest={onTest} t={t} /> : null}
      </div>
    </section>
  )
}

export default memo(AiProviderSettings)
