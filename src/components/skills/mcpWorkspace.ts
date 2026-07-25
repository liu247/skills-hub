import type { McpServerDto } from './types'

export type McpSourceCollection = {
  key: string
  label: string
  sourceUrl: string | null
  serviceCount: number
  updatedAt: number | null
  servers: McpServerDto[]
}

const manualSource = {
  key: 'manual',
  label: 'Manual configuration',
  sourceUrl: null,
}

const sourceIdentity = (sourceUrl: string | null | undefined) => {
  if (!sourceUrl) return manualSource
  const normalized = sourceUrl.replace(/\/$/, '')
  try {
    const parsed = new URL(normalized)
    const parts = parsed.pathname.split('/').filter(Boolean)
    if (parsed.hostname.includes('github.com') && parts.length >= 2) {
      return {
        key: normalized,
        label: `${parts[0]}/${parts[1].replace(/\.git$/, '')}`,
        sourceUrl,
      }
    }
  } catch {
    // Keep non-URL source metadata visible without blocking the workspace.
  }
  return { key: normalized, label: normalized, sourceUrl }
}

const latestSyncAt = (server: McpServerDto) =>
  server.targets.reduce<number | null>((latest, target) => {
    if (!target.synced_at) return latest
    return latest === null || target.synced_at > latest ? target.synced_at : latest
  }, null)

export const groupMcpServersBySource = (servers: McpServerDto[]): McpSourceCollection[] => {
  const groups = new Map<string, McpSourceCollection>()
  for (const server of servers) {
    const source = sourceIdentity(server.source_url)
    const existing = groups.get(source.key)
    const updatedAt = latestSyncAt(server)
    if (existing) {
      existing.servers.push(server)
      existing.serviceCount += 1
      if (updatedAt !== null && (existing.updatedAt === null || updatedAt > existing.updatedAt)) {
        existing.updatedAt = updatedAt
      }
      continue
    }
    groups.set(source.key, {
      ...source,
      serviceCount: 1,
      updatedAt,
      servers: [server],
    })
  }
  return [...groups.values()].sort((left, right) =>
    right.serviceCount - left.serviceCount || left.label.localeCompare(right.label),
  )
}
