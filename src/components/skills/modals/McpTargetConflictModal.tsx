import { memo } from 'react'
import type { TFunction } from 'i18next'

type Props = {
  open: boolean
  busy: boolean
  tools: string[]
  serverName: string
  onCancel: () => void
  onReplace: () => void
  t: TFunction
}

const McpTargetConflictModal = ({ open, busy, tools, serverName, onCancel, onReplace, t }: Props) => {
  if (!open) return null
  const apps = tools.map((tool) => t(`tools.${tool}`)).join(', ')
  return <div className="modal-backdrop" onClick={onCancel}><div className="modal modal-sm" role="dialog" aria-modal="true" aria-labelledby="mcp-target-conflict-title" onClick={(event) => event.stopPropagation()}><div className="modal-header"><div><div className="modal-title" id="mcp-target-conflict-title">{t('mcp.targetConflictTitle')}</div><p className="modal-subtitle">{t('mcp.targetConflictBody', { apps, name: serverName })}</p></div><button className="modal-close" type="button" onClick={onCancel} aria-label={t('close')}>✕</button></div><div className="modal-footer"><button className="btn btn-secondary" disabled={busy} onClick={onCancel}>{t('mcp.keepLocal')}</button><button className="btn btn-primary" disabled={busy} onClick={onReplace}>{t('mcp.replaceAndManage')}</button></div></div></div>
}

export default memo(McpTargetConflictModal)
