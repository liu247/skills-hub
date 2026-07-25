import { memo, useState } from 'react'
import { Github, Plus, Search } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { McpImportCandidateDto } from './types'

type Props = {
  busy: boolean
  candidates: McpImportCandidateDto[]
  onScan: (url: string) => void
  onImport: (candidates: McpImportCandidateDto[]) => void
  onOpenManual: () => void
  t: TFunction
}

const McpImportPage = ({ busy, candidates, onScan, onImport, onOpenManual, t }: Props) => {
  const [url, setUrl] = useState('')
  const [selected, setSelected] = useState<Record<string, boolean>>({})
  const key = (candidate: McpImportCandidateDto) => `${candidate.source_path}:${candidate.name}`
  const selectedCandidates = candidates.filter((candidate) => selected[key(candidate)])

  return <div className="mcp-page">
    <div className="mcp-page-head"><div><h2>{t('mcpImport.title')}</h2><p>{t('mcpImport.help')}</p></div><button type="button" className="btn btn-secondary" onClick={onOpenManual}><Plus size={15}/>{t('mcpImport.manual')}</button></div>
    <div className="mcp-import-search"><Github size={18}/><input value={url} onChange={(event) => setUrl(event.target.value)} placeholder={t('mcpImport.url')}/><button type="button" className="btn btn-primary" disabled={busy || !url.trim()} onClick={() => onScan(url)}><Search size={15}/>{t('mcpImport.scan')}</button></div>
    {!candidates.length ? <p className="empty-state">{t('mcpImport.empty')}</p> : <><div className="mcp-import-list">{candidates.map((candidate) => <label className="mcp-server-card" key={key(candidate)}><input type="checkbox" checked={Boolean(selected[key(candidate)])} onChange={(event) => setSelected({ ...selected, [key(candidate)]: event.target.checked })}/><div><strong>{candidate.name}</strong><span className="mcp-transport">{candidate.transport}</span><p>{candidate.source_path}</p></div></label>)}</div><button type="button" className="btn btn-primary" disabled={busy || !selectedCandidates.length} onClick={() => onImport(selectedCandidates)}>{t('mcpImport.importSelected')}</button></>}
  </div>
}

export default memo(McpImportPage)
