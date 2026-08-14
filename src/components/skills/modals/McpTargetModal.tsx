import { memo, useState } from 'react'
import type { TFunction } from 'i18next'

const targets = ['codex', 'claude_code', 'claude_3p', 'kiro', 'reasonix', 'custom_deepseek_harness']

type Props = { open: boolean; busy: boolean; selectedTools: string[]; onClose: () => void; onSave: (tools: string[]) => void; t: TFunction }

const McpTargetModal = ({ open, busy, selectedTools, onClose, onSave, t }: Props) => {
  const [selected, setSelected] = useState(() => new Set(selectedTools))
  if (!open) return null
  const toggle = (tool: string) => setSelected((current) => {
    const next = new Set(current)
    if (next.has(tool)) next.delete(tool); else next.add(tool)
    return next
  })
  return <div className="modal-backdrop" onClick={onClose}><div className="modal modal-sm mcp-target-modal" onClick={(event) => event.stopPropagation()}><div className="modal-header"><div><div className="modal-title">{t('mcp.targetsTitle')}</div><p className="modal-subtitle">{t('mcp.targetsHelp')}</p></div><button className="modal-close" type="button" onClick={onClose}>✕</button></div><div className="modal-body mcp-target-options">{targets.map((tool) => <label key={tool} className="mcp-target-option"><input type="checkbox" checked={selected.has(tool)} onChange={() => toggle(tool)}/><span>{t(`tools.${tool}`)}</span></label>)}</div><div className="modal-footer"><button className="btn btn-secondary" disabled={busy} onClick={onClose}>{t('cancel')}</button><button className="btn btn-primary" disabled={busy} onClick={() => onSave([...selected])}>{t('mcp.saveTargets')}</button></div></div></div>
}

export default memo(McpTargetModal)
