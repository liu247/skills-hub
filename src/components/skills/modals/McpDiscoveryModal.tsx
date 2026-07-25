import { memo, useState } from 'react'
import { Download } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { LocalMcpPlanDto } from '../types'

type Props = { open: boolean; plan: LocalMcpPlanDto; loading: boolean; onClose: () => void; onImport: (selected: Record<string, number>) => void; t: TFunction }

const McpDiscoveryModal = ({ open, plan, loading, onClose, onImport, t }: Props) => {
  const [selected, setSelected] = useState<Record<string, number>>({})
  if (!open) return null
  const count = Object.keys(selected).length
  return <div className="modal-backdrop" onClick={onClose}><div className="modal modal-lg modal-discovered" onClick={(event) => event.stopPropagation()}><div className="modal-header"><div className="modal-title">{t('mcp.localReviewTitle')}</div><button className="modal-close" type="button" onClick={onClose}>✕</button></div><div className="modal-body"><div className="import-summary">{t('mcp.localReviewSummary', { hosts: plan.total_hosts_scanned, count: plan.total_servers_found })}</div><p className="mcp-credential-migration">{t('mcp.localCredentialMigration')}</p><div className="groups discovered-list">{plan.groups.map((group) => <div className="group-card" key={group.name}><div className="group-title"><label className="group-select"><input type="checkbox" checked={selected[group.name] !== undefined} onChange={(event) => setSelected(event.target.checked ? { ...selected, [group.name]: selected[group.name] ?? 0 } : Object.fromEntries(Object.entries(selected).filter(([name]) => name !== group.name)))}/><span>{group.name}</span></label>{group.has_conflict ? <span className="badge danger">{t('conflict')}</span> : <span className="badge">{t('consistent')}</span>}</div>{group.variants.map((variant, index) => <label className="variant-row" key={`${variant.host}-${variant.path}`}><input type="radio" name={group.name} checked={(selected[group.name] ?? 0) === index} onChange={() => setSelected({ ...selected, [group.name]: index })}/><div className="variant-info"><span className="path">{variant.path}</span><span className="found-pill">{variant.host}</span>{variant.credential_names.length ? <span className="mcp-discovered-credentials">{variant.credential_names.join(', ')}</span> : null}</div></label>)}</div>)}</div></div><div className="modal-footer"><button className="btn btn-primary" disabled={loading || count === 0} onClick={() => onImport(selected)}><Download size={14}/>{t('mcp.adoptSelected', { count })}</button><button className="btn btn-secondary" disabled={loading} onClick={onClose}>{t('close')}</button></div></div></div>
}
export default memo(McpDiscoveryModal)
