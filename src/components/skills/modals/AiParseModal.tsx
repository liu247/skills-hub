import { memo, useMemo, useState } from 'react'
import { Bot, FileSearch, TriangleAlert } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { AiParsePlanDto, AiProviderConfigDto } from '../types'

type AiParseModalProps = {
  open: boolean
  mode: 'skill' | 'mcp'
  busy: boolean
  providers: AiProviderConfigDto[]
  onClose: () => void
  onParse: (provider: AiProviderConfigDto['provider'], sourceUrl: string) => Promise<AiParsePlanDto | null>
  onConfirm: (plan: AiParsePlanDto) => void
  t: TFunction
}

const AiParseModal = ({ open, mode, busy, providers, onClose, onParse, onConfirm, t }: AiParseModalProps) => {
  const eligible = useMemo(() => providers.filter((provider) => provider.enabled && provider.has_api_key), [providers])
  const [sourceUrl, setSourceUrl] = useState('')
  const [provider, setProvider] = useState<AiProviderConfigDto['provider'] | ''>('')
  const [plan, setPlan] = useState<AiParsePlanDto | null>(null)
  const [planText, setPlanText] = useState('')
  const [planError, setPlanError] = useState('')

  if (!open) return null

  const parse = async () => {
    const selectedProvider = provider || eligible[0]?.provider
    if (!selectedProvider || !sourceUrl.trim()) return
    const nextPlan = await onParse(selectedProvider, sourceUrl.trim())
    setPlan(nextPlan)
    setPlanText(nextPlan ? JSON.stringify(nextPlan, null, 2) : '')
    setPlanError('')
  }

  const updatePlan = (value: string) => {
    setPlanText(value)
    try {
      setPlan(JSON.parse(value) as AiParsePlanDto)
      setPlanError('')
    } catch {
      setPlanError(t('aiParse.invalidPlan'))
    }
  }

  return <div className="modal-backdrop" onClick={() => !busy && onClose()}>
    <div className="modal ai-parse-modal" onClick={(event) => event.stopPropagation()}>
      <div className="modal-header">
        <div className="modal-title"><Bot size={18}/>{t('aiParse.title')}</div>
        <button className="modal-close" type="button" disabled={busy} onClick={onClose} aria-label={t('close')}>✕</button>
      </div>
      <div className="modal-body ai-parse-body">
        {!eligible.length ? <div className="ai-parse-notice"><TriangleAlert size={16}/>{t('aiParse.noProvider')}</div> : <>
          <label className="form-field"><span className="label">{t('aiParse.sourceUrl')}</span><input className="input mono" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} placeholder="https://github.com/owner/repository"/></label>
          <label className="form-field"><span className="label">{t('aiParse.provider')}</span><select className="input" value={provider || eligible[0]?.provider || ''} onChange={(event) => setProvider(event.target.value as AiProviderConfigDto['provider'])}>{eligible.map((item) => <option key={item.provider} value={item.provider}>{item.provider} · {item.model}</option>)}</select></label>
          {!plan ? <button type="button" className="btn btn-primary" disabled={busy || !sourceUrl.trim() || !eligible.length} onClick={() => void parse()}><FileSearch size={15}/>{t('aiParse.parse')}</button> : <div className="ai-parse-preview">
            <div className="ai-parse-summary"><strong>{plan.kind === 'unknown' ? t('aiParse.unknown') : plan.kind.toUpperCase()}</strong><span>{plan.confidence}</span><p>{plan.summary}</p></div>
            <div className="ai-parse-evidence"><strong>{t('aiParse.evidence')}</strong><ul>{plan.source.evidence.map((item, index) => <li key={`${index}-${item}`}>{item}</li>)}</ul></div>
            {plan.warnings.length ? <div className="ai-parse-notice"><TriangleAlert size={16}/><span>{plan.warnings.join(' ')}</span></div> : null}
            <label className="form-field"><span className="label">{t('aiParse.editPlan')}</span><textarea className="input mono ai-parse-json" value={planText} onChange={(event) => updatePlan(event.target.value)} /></label>
            {planError ? <div className="ai-parse-notice"><TriangleAlert size={16}/>{planError}</div> : null}
            {plan.kind === mode ? <button type="button" className="btn btn-primary" disabled={busy || Boolean(planError)} onClick={() => onConfirm(plan)}>{t('aiParse.confirm')}</button> : <div className="ai-parse-notice"><TriangleAlert size={16}/>{t('aiParse.wrongKind')}</div>}
          </div>}
        </>}
      </div>
    </div>
  </div>
}

export default memo(AiParseModal)
