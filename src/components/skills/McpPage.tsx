import { memo, useState } from 'react'
import { Plus, RefreshCw, Trash2 } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { McpServerDto } from './types'

type McpPageProps = {
  servers: McpServerDto[]
  busy: boolean
  onSave: (server: McpServerDto) => Promise<McpServerDto | null>
  onSetSecret: (serverId: string, envVar: string, value: string) => Promise<void>
  onDelete: (serverId: string) => Promise<void>
  onSync: (serverId: string, tools: string[]) => Promise<void>
  t: TFunction
}

const targets = ['codex', 'claude_code', 'kiro', 'reasonix']

const emptyServer = (): McpServerDto => ({
  id: '', name: '', transport: 'stdio', command: '', args: [], env: {}, cwd: null,
  url: '', headers: {}, enabled: true, proxy_enabled: true, secret_refs: [], targets: [],
})

const McpPage = ({ servers, busy, onSave, onSetSecret, onDelete, onSync, t }: McpPageProps) => {
  const [draft, setDraft] = useState<McpServerDto | null>(null)
  const [secretNames, setSecretNames] = useState('')
  const [headerName, setHeaderName] = useState('Authorization')

  const save = async () => {
    if (!draft) return
    const names = secretNames.split(',').map((name) => name.trim()).filter(Boolean)
    const env = Object.fromEntries(names.map((name) => [name, `\${${name}}`]))
    const headers = draft.transport === 'http' && names[0]
      ? { [headerName.trim() || 'Authorization']: `\${${names[0]}}` }
      : {}
    const saved = await onSave({ ...draft, env: draft.transport === 'stdio' ? env : {}, headers, secret_refs: names.map((env_var) => ({ env_var, has_value: false })) })
    if (saved) {
      for (const name of names) {
        const value = window.prompt(t('mcp.secretPrompt', { name }))
        if (value) await onSetSecret(saved.id, name, value)
      }
    }
    setDraft(null)
    setSecretNames('')
  }

  return <div className="mcp-page">
    <div className="mcp-page-head">
      <div><h2>{t('mcp.title')}</h2><p>{t('mcp.help')}</p></div>
      <button type="button" className="primary-btn" onClick={() => setDraft(emptyServer())}><Plus size={16}/>{t('mcp.add')}</button>
    </div>
    {draft ? <div className="mcp-editor">
      <label>{t('mcp.name')}<input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })}/></label>
      <label>{t('mcp.transport')}<select value={draft.transport} onChange={(event) => setDraft({ ...draft, transport: event.target.value })}><option value="stdio">stdio</option><option value="http">HTTP</option></select></label>
      {draft.transport === 'stdio' ? <><label>{t('mcp.command')}<input value={draft.command ?? ''} onChange={(event) => setDraft({ ...draft, command: event.target.value })}/></label><label>{t('mcp.args')}<input value={draft.args.join(' ')} onChange={(event) => setDraft({ ...draft, args: event.target.value.split(' ').filter(Boolean) })}/></label></> : <><label>{t('mcp.url')}<input value={draft.url ?? ''} onChange={(event) => setDraft({ ...draft, url: event.target.value })}/></label><label>{t('mcp.headerName')}<input value={headerName} onChange={(event) => setHeaderName(event.target.value)}/></label></>}
      <label>{t('mcp.secrets')}<input placeholder="API_TOKEN, GITHUB_TOKEN" value={secretNames} onChange={(event) => setSecretNames(event.target.value)}/></label>
      <div className="mcp-editor-actions"><button type="button" onClick={() => setDraft(null)}>{t('cancel')}</button><button type="button" className="primary-btn" disabled={busy} onClick={() => void save()}>{t('save')}</button></div>
    </div> : null}
    <div className="mcp-server-list">
      {servers.map((server) => <article className="mcp-server-card" key={server.id}>
        <div><strong>{server.name}</strong><span className="mcp-transport">{server.transport}</span><p>{server.transport === 'stdio' ? `${server.command ?? ''} ${server.args.join(' ')}` : server.url}</p><small>{server.secret_refs.length ? t('mcp.secretStatus', { count: server.secret_refs.filter((item) => item.has_value).length, total: server.secret_refs.length }) : t('mcp.noSecrets')}</small></div>
        <div className="mcp-server-actions"><button type="button" disabled={busy} onClick={() => void onSync(server.id, targets)}><RefreshCw size={15}/>{t('mcp.sync')}</button><button type="button" className="icon-btn danger" onClick={() => void onDelete(server.id)} aria-label={t('delete')}><Trash2 size={16}/></button></div>
      </article>)}
      {!servers.length ? <p className="empty-state">{t('mcp.empty')}</p> : null}
    </div>
  </div>
}

export default memo(McpPage)
