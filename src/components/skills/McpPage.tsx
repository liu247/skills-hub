import { memo, useMemo, useState } from 'react'
import { ArrowUpDown, ChevronLeft, FolderKanban, Grid2X2, List, Plus, RefreshCw, Search, Server, Trash2 } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { McpServerDto } from './types'
import { groupMcpServersBySource } from './mcpWorkspace'
import McpTargetModal from './modals/McpTargetModal'

type McpPageProps = {
  servers: McpServerDto[]
  busy: boolean
  initialManualEditor: boolean
  onSave: (server: McpServerDto) => Promise<McpServerDto | null>
  onSetSecret: (serverId: string, envVar: string, value: string) => Promise<void>
  onDelete: (serverId: string) => Promise<void>
  onSync: (serverId: string, tools: string[]) => Promise<void>
  onSetTargets: (serverId: string, tools: string[]) => Promise<void>
  onScanLocal: () => void
  onOpenImport: () => void
  onCloseManualEditor: () => void
  t: TFunction
}

const targets = ['codex', 'claude_code', 'kiro', 'reasonix']

const emptyServer = (): McpServerDto => ({
  id: '', name: '', transport: 'stdio', command: '', args: [], env: {}, cwd: null,
  url: '', headers: {}, enabled: true, proxy_enabled: true, source_url: null, source_path: null, secret_refs: [], targets: [],
})

const McpPage = ({ servers, busy, initialManualEditor, onSave, onSetSecret, onDelete, onSync, onSetTargets, onScanLocal, onOpenImport, onCloseManualEditor, t }: McpPageProps) => {
  const [draft, setDraft] = useState<McpServerDto | null>(() => initialManualEditor ? emptyServer() : null)
  const [secretNames, setSecretNames] = useState('')
  const [headerName, setHeaderName] = useState('Authorization')
  const [query, setQuery] = useState('')
  const [sourceFilter, setSourceFilter] = useState('all')
  const [sortBy, setSortBy] = useState<'updated' | 'name'>('updated')
  const [viewMode, setViewMode] = useState<'cards' | 'list'>('cards')
  const [activeSource, setActiveSource] = useState<string | null>(null)
  const [targetServer, setTargetServer] = useState<McpServerDto | null>(null)
  const collections = useMemo(() => groupMcpServersBySource(servers, (host) => t('mcp.localSource', { app: host === 'claude_code' ? 'Claude Code' : host === 'reasonix' ? 'Reasonix' : host === 'kiro' ? 'Kiro' : 'Codex' })), [servers, t])

  const credentialTotal = servers.reduce((total, server) => total + server.secret_refs.length, 0)
  const credentialReady = servers.reduce((total, server) => total + server.secret_refs.filter((secret) => secret.has_value).length, 0)
  const syncedApps = new Set(servers.flatMap((server) => server.targets.map((target) => target.tool))).size
  const syncHealthy = servers.every((server) => server.targets.every((target) => target.status !== 'error'))
  const normalizedQuery = query.trim().toLowerCase()
  const filteredCollections = collections
    .filter((collection) => sourceFilter === 'all' || collection.key === sourceFilter)
    .filter((collection) => !normalizedQuery || collection.label.toLowerCase().includes(normalizedQuery) || collection.servers.some((server) => server.name.toLowerCase().includes(normalizedQuery)))
    .sort((left, right) => sortBy === 'name' ? left.label.localeCompare(right.label) : (right.updatedAt ?? 0) - (left.updatedAt ?? 0) || left.label.localeCompare(right.label))
  const currentCollection = collections.find((collection) => collection.key === activeSource) ?? null

  const save = async () => {
    if (!draft) return
    const names = secretNames.split(',').map((name) => name.trim()).filter(Boolean)
    const env = Object.fromEntries(names.map((name) => [name, `\${${name}}`]))
    const headers = draft.transport === 'http' && names[0] ? { [headerName.trim() || 'Authorization']: `\${${names[0]}}` } : {}
    const saved = await onSave({ ...draft, env: draft.transport === 'stdio' ? env : {}, headers, secret_refs: names.map((env_var) => ({ env_var, has_value: false })) })
    if (saved) for (const name of names) {
      const value = window.prompt(t('mcp.secretPrompt', { name }))
      if (value) await onSetSecret(saved.id, name, value)
    }
    setDraft(null)
    if (initialManualEditor) onCloseManualEditor()
    setSecretNames('')
  }

  const sourceLabel = (key: string) => key === 'manual' ? t('mcp.manualSource') : collections.find((collection) => collection.key === key)?.label ?? key

  const renderEditor = () => draft ? <div className="mcp-editor">
    <label>{t('mcp.name')}<input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })}/></label>
    <label>{t('mcp.transport')}<select value={draft.transport} onChange={(event) => setDraft({ ...draft, transport: event.target.value })}><option value="stdio">stdio</option><option value="http">HTTP</option></select></label>
    {draft.transport === 'stdio' ? <><label>{t('mcp.command')}<input value={draft.command ?? ''} onChange={(event) => setDraft({ ...draft, command: event.target.value })}/></label><label>{t('mcp.args')}<input value={draft.args.join(' ')} onChange={(event) => setDraft({ ...draft, args: event.target.value.split(' ').filter(Boolean) })}/></label></> : <><label>{t('mcp.url')}<input value={draft.url ?? ''} onChange={(event) => setDraft({ ...draft, url: event.target.value })}/></label><label>{t('mcp.headerName')}<input value={headerName} onChange={(event) => setHeaderName(event.target.value)}/></label></>}
    <label>{t('mcp.secrets')}<input placeholder="API_TOKEN, GITHUB_TOKEN" value={secretNames} onChange={(event) => setSecretNames(event.target.value)}/></label>
    <div className="mcp-editor-actions"><button type="button" className="btn btn-secondary" onClick={() => { setDraft(null); if (initialManualEditor) onCloseManualEditor() }}>{t('cancel')}</button><button type="button" className="btn btn-primary" disabled={busy} onClick={() => void save()}>{t('save')}</button></div>
  </div> : null

  const renderServer = (server: McpServerDto) => <article className="mcp-server-card" key={server.id}>
    <div className="mcp-server-info"><div className="mcp-server-title"><Server size={17}/><strong>{server.name}</strong><span className="mcp-transport">{server.transport}</span></div><p>{server.transport === 'stdio' ? `${server.command ?? ''} ${server.args.join(' ')}` : server.url}</p><div className="mcp-target-badges">{server.targets.length ? server.targets.map((target) => <span className={target.status === 'error' ? 'mcp-target-badge error' : 'mcp-target-badge'} key={target.tool}>{t(target.tool)}</span>) : <span className="mcp-target-empty">{t('mcp.noTargets')}</span>}</div>{server.source_path ? <small>{server.source_path}</small> : null}<small>{server.secret_refs.length ? t('mcp.secretStatus', { count: server.secret_refs.filter((item) => item.has_value).length, total: server.secret_refs.length }) : t('mcp.noSecrets')}</small></div>
    <div className="mcp-server-actions"><button type="button" className="btn btn-secondary" disabled={busy} onClick={() => setTargetServer(server)}>{t('mcp.manageTargets')}</button><button type="button" className="icon-btn danger" onClick={() => void onDelete(server.id)} aria-label={t('delete')}><Trash2 size={16}/></button></div>
  </article>

  const targetModal = targetServer ? <McpTargetModal open busy={busy} selectedTools={targetServer.targets.map((target) => target.tool)} onClose={() => setTargetServer(null)} onSave={(tools) => { void onSetTargets(targetServer.id, tools); setTargetServer(null) }} t={t} /> : null

  if (currentCollection) return <><div className="mcp-workspace"><div className="collection-breadcrumb"><button type="button" className="btn btn-secondary" onClick={() => setActiveSource(null)}><ChevronLeft size={15}/>{t('mcp.back')}</button><span className="collection-breadcrumb-name">{sourceLabel(currentCollection.key)}</span><button type="button" className="btn btn-secondary mcp-source-sync" disabled={busy} onClick={() => void Promise.all(currentCollection.servers.map((server) => onSync(server.id, targets)))}><RefreshCw size={15}/>{t('mcp.sourceSync')}</button></div>{renderEditor()}<div className="mcp-server-list">{currentCollection.servers.map(renderServer)}</div></div>{targetModal}</>

  return <><div className="mcp-workspace">
    <section className="dashboard-stats" aria-label={t('mcp.title')}>
      <article><span>{t('mcp.managed')}</span><strong>{servers.length}</strong></article>
      <article><span>{t('mcp.apps')}</span><strong>{syncedApps}</strong></article>
      <article><span>{t('mcp.credentials')}</span><strong>{credentialTotal ? `${credentialReady}/${credentialTotal}` : '—'}</strong></article>
      <article><span>{t('mcp.syncStatus')}</span><strong className="status-summary"><i />{syncHealthy ? t('mcp.allNormal') : t('mcp.needsAttention')}</strong></article>
    </section>
    <div className="mcp-filter-bar">
      <div className="search-container"><Search size={16} className="search-icon-abs"/><input className="search-input" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t('mcp.sourceSearch')}/></div>
      <button className="btn btn-secondary sort-btn" type="button"><span>{sourceFilter === 'all' ? t('mcp.allSources') : sourceLabel(sourceFilter)}</span><select aria-label={t('mcp.sourceFilter')} value={sourceFilter} onChange={(event) => setSourceFilter(event.target.value)}><option value="all">{t('mcp.allSources')}</option>{collections.map((collection) => <option value={collection.key} key={collection.key}>{sourceLabel(collection.key)}</option>)}</select></button>
      <button className="btn btn-secondary sort-btn" type="button"><ArrowUpDown size={14}/><span>{sortBy === 'updated' ? t('mcp.sortUpdated') : t('mcp.sortName')}</span><select aria-label={t('mcp.sort')} value={sortBy} onChange={(event) => setSortBy(event.target.value as 'updated' | 'name')}><option value="updated">{t('mcp.sortUpdated')}</option><option value="name">{t('mcp.sortName')}</option></select></button>
      <button type="button" className="btn btn-secondary" disabled={busy || !servers.length} onClick={() => void Promise.all(servers.map((server) => onSync(server.id, targets)))}><RefreshCw size={15}/>{t('mcp.batchSync')}</button>
      <button type="button" className="btn btn-secondary" disabled={busy} onClick={onScanLocal}><Search size={15}/>{t('mcp.scanLocal')}</button>
      <div className="view-mode-toggle" role="group" aria-label={t('mcp.viewMode')}><button className={viewMode === 'cards' ? 'active' : ''} type="button" onClick={() => setViewMode('cards')} aria-label={t('mcp.gridView')}><Grid2X2 size={15}/></button><button className={viewMode === 'list' ? 'active' : ''} type="button" onClick={() => setViewMode('list')} aria-label={t('mcp.listView')}><List size={15}/></button></div>
      <button type="button" className="btn btn-primary mcp-add-button" onClick={onOpenImport}><Plus size={16}/>{t('mcp.add')}</button>
    </div>
    {renderEditor()}
    {!filteredCollections.length ? <div className="empty">{t('mcp.empty')}</div> : <div className={`collections-list mcp-collections ${viewMode}-view`}><div className="collections-grid">{filteredCollections.map((collection) => <button type="button" className="collection-card mcp-source-card" key={collection.key} onClick={() => setActiveSource(collection.key)}><span className="collection-card-head"><span className="collection-card-icon" aria-hidden="true"><FolderKanban size={18}/></span><span className="collection-card-name">{sourceLabel(collection.key)}</span></span><span className="collection-card-meta"><span>{t('mcp.sourceCount', { count: collection.serviceCount })}</span><span>{collection.updatedAt ? new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(collection.updatedAt) : t('mcp.notSynced')}</span></span></button>)}</div></div>}
  </div>{targetModal}</>
}

export default memo(McpPage)
