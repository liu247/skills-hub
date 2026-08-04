import { memo, useState } from 'react'
import { Bot, Github, Plus, Search, Server, Star } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { GithubRepoSummaryDto, McpImportCandidateDto } from './types'

type Props = {
  busy: boolean
  candidates: McpImportCandidateDto[]
  searchResults: GithubRepoSummaryDto[]
  onSearch: (query: string) => void
  onScan: (url: string) => void
  onImport: (candidates: McpImportCandidateDto[]) => void
  onOpenManual: () => void
  onOpenAiParse: () => void
  t: TFunction
}

const McpImportPage = ({ busy, candidates, searchResults, onSearch, onScan, onImport, onOpenManual, onOpenAiParse, t }: Props) => {
  const [url, setUrl] = useState('')
  const [searchQuery, setSearchQuery] = useState('')
  const [selected, setSelected] = useState<Record<string, boolean>>({})
  const key = (candidate: McpImportCandidateDto) => `${candidate.source_path}:${candidate.name}`
  const selectedCandidates = candidates.filter((candidate) => selected[key(candidate)])

  return <div className="mcp-import-workbench">
    <div className="mcp-import-heading">
      <div><h1>{t('mcpImport.title')}</h1><p>{t('mcpImport.help')}</p></div>
      <div className="mcp-import-heading-actions"><button type="button" className="btn btn-secondary" onClick={onOpenAiParse}><Bot size={15}/>{t('aiParse.open')}</button><button type="button" className="btn btn-secondary" onClick={onOpenManual}><Plus size={15}/>{t('mcpImport.manual')}</button></div>
    </div>
    <section className="mcp-import-toolbar" aria-label={t('mcpImport.searchTitle')}>
      <span className="mcp-import-github" aria-hidden="true"><Search size={19}/></span>
      <div className="mcp-import-input"><label htmlFor="mcp-name-search">{t('mcpImport.searchTitle')}</label><input id="mcp-name-search" value={searchQuery} onChange={(event) => setSearchQuery(event.target.value)} placeholder={t('mcpImport.searchPlaceholder')}/></div>
      <button type="button" className="btn btn-primary" disabled={busy || !searchQuery.trim()} onClick={() => onSearch(searchQuery)}>{t('mcpImport.searchButton')}</button>
    </section>
    {searchResults.length ? <div className="mcp-search-results">
      <div className="mcp-search-results-head"><strong>{t('mcpImport.searchResults')}</strong><span>{t('mcpImport.searchHint')}</span></div>
      <div className="mcp-search-repo-grid">{searchResults.map((repo) => <button type="button" className="mcp-search-repo" key={repo.html_url} disabled={busy} title={t('mcpImport.scanRepo')} onClick={() => { setSelected({}); onScan(repo.html_url) }}><span className="mcp-search-repo-icon"><Github size={16}/></span><span className="mcp-search-repo-copy"><strong>{repo.full_name}</strong><small>{repo.description || repo.clone_url}</small></span><span className="mcp-search-repo-stars"><Star size={13}/>{repo.stars}</span></button>)}</div>
    </div> : null}
    <section className="mcp-import-toolbar" aria-label={t('mcpImport.github')}>
      <span className="mcp-import-github" aria-hidden="true"><Github size={19}/></span>
      <div className="mcp-import-input"><label htmlFor="mcp-source-url">{t('mcpImport.github')}</label><input id="mcp-source-url" value={url} onChange={(event) => setUrl(event.target.value)} placeholder={t('mcpImport.url')}/></div>
      <button type="button" className="btn btn-secondary" disabled={busy || !url.trim()} onClick={() => onScan(url)}><Search size={15}/>{t('mcpImport.scan')}</button>
    </section>
    {!candidates.length ? <div className="mcp-import-empty"><Server size={20}/><div><strong>{t('mcpImport.emptyTitle')}</strong><p>{t('mcpImport.empty')}</p></div></div> : <div className="mcp-import-results"><div className="mcp-import-results-head"><div><strong>{t('mcpImport.resultsTitle')}</strong><span>{t('mcpImport.resultsHint')}</span></div><button type="button" className="btn btn-primary" disabled={busy || !selectedCandidates.length} onClick={() => onImport(selectedCandidates)}>{t('mcpImport.importSelected')}</button></div><div className="mcp-import-candidate-grid">{candidates.map((candidate) => <label className={`mcp-import-candidate${selected[key(candidate)] ? ' selected' : ''}`} key={key(candidate)}><input type="checkbox" checked={Boolean(selected[key(candidate)])} onChange={(event) => setSelected({ ...selected, [key(candidate)]: event.target.checked })}/><span className="mcp-import-candidate-icon"><Server size={17}/></span><span className="mcp-import-candidate-copy"><strong>{candidate.name}</strong><span><em>{candidate.transport}</em>{candidate.source_path}</span></span></label>)}</div></div>}
  </div>
}

export default memo(McpImportPage)
