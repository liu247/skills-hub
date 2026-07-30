import { memo, useEffect, useState } from 'react'
import { EyeOff, KeyRound, Plus, Trash2 } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { CollectionParameterDto } from '../types'

type Props = {
  open: boolean
  collectionName: string | null
  parameters: CollectionParameterDto[]
  onClose: () => void
  onSave: (parameter: CollectionParameterDto, secretValue?: string) => Promise<void>
  onDelete: (name: string) => Promise<void>
  t: TFunction
}

const empty = (): CollectionParameterDto => ({ name: '', description: '', is_sensitive: false, value: '', has_value: false })

const CollectionParametersModal = ({ open, collectionName, parameters, onClose, onSave, onDelete, t }: Props) => {
  const [draft, setDraft] = useState<CollectionParameterDto | null>(null)
  const [secretValue, setSecretValue] = useState('')
  const [busy, setBusy] = useState(false)

  useEffect(() => { if (open) { setDraft(null); setSecretValue('') } }, [open, collectionName])
  if (!open || !collectionName) return null
  const begin = (parameter: CollectionParameterDto) => { setDraft({ ...parameter, value: parameter.value ?? '' }); setSecretValue('') }
  const save = async () => {
    if (!draft) return
    setBusy(true)
    try { await onSave(draft, draft.is_sensitive ? secretValue : undefined); setDraft(null); setSecretValue('') } finally { setBusy(false) }
  }
  return <div className="modal-backdrop" onClick={() => !busy && onClose()}>
    <div className="modal collection-parameters-modal" onClick={(event) => event.stopPropagation()}>
      <div className="modal-header"><div className="modal-title"><KeyRound size={18}/>{t('collectionParameters.title', { name: collectionName })}</div><button className="modal-close" type="button" onClick={onClose} disabled={busy} aria-label={t('close')}>✕</button></div>
      <div className="modal-body">
        <p className="modal-description">{t('collectionParameters.description')}</p>
        <div className="collection-parameter-list">{parameters.map((parameter) => <div className="collection-parameter-row" key={parameter.name}>
          <div><strong>{parameter.name}</strong><span>{parameter.description || t('collectionParameters.noDescription')}</span></div>
          <span className={parameter.has_value ? 'parameter-status configured' : 'parameter-status'}>{parameter.is_sensitive ? <EyeOff size={14}/> : null}{parameter.has_value ? t('collectionParameters.configured') : t('collectionParameters.notConfigured')}</span>
          <button className="btn btn-secondary btn-sm" type="button" onClick={() => begin(parameter)}>{t('edit')}</button>
          <button className="icon-btn danger" type="button" aria-label={t('delete')} onClick={() => void onDelete(parameter.name)}><Trash2 size={14}/></button>
        </div>)}</div>
        {draft ? <div className="collection-parameter-editor">
          <label className="form-field"><span className="label">{t('collectionParameters.name')}</span><input className="input mono" value={draft.name} disabled={parameters.some((item) => item.name === draft.name)} onChange={(event) => setDraft({ ...draft, name: event.target.value.toUpperCase() })}/></label>
          <label className="form-field"><span className="label">{t('collectionParameters.descriptionLabel')}</span><input className="input" value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })}/></label>
          <label className="checkbox-row"><input type="checkbox" checked={draft.is_sensitive} onChange={(event) => setDraft({ ...draft, is_sensitive: event.target.checked, value: event.target.checked ? null : '' })}/>{t('collectionParameters.sensitive')}</label>
          {draft.is_sensitive ? <label className="form-field"><span className="label">{draft.has_value ? t('collectionParameters.replaceSecret') : t('collectionParameters.secretValue')}</span><input className="input" type="password" value={secretValue} onChange={(event) => setSecretValue(event.target.value)} placeholder={draft.has_value ? t('collectionParameters.leaveUnchanged') : ''}/></label> : <label className="form-field"><span className="label">{t('collectionParameters.value')}</span><input className="input" value={draft.value ?? ''} onChange={(event) => setDraft({ ...draft, value: event.target.value })}/></label>}
          <div className="modal-actions"><button className="btn btn-secondary" type="button" onClick={() => setDraft(null)}>{t('cancel')}</button><button className="btn btn-primary" type="button" disabled={busy || !draft.name.trim()} onClick={() => void save()}>{t('save')}</button></div>
        </div> : <button className="btn btn-secondary" type="button" onClick={() => begin(empty())}><Plus size={15}/>{t('collectionParameters.add')}</button>}
      </div>
    </div>
  </div>
}

export default memo(CollectionParametersModal)
