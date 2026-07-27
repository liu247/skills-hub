import { memo, useMemo, useState } from 'react'
import { Bot, ChevronDown, ChevronRight, FolderKanban, Plus, Search, Star } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { FeaturedSkillDto, ManagedSkill, OnlineSkillDto } from './types'

type ExplorePageProps = {
  featuredSkills: FeaturedSkillDto[]
  featuredLoading: boolean
  exploreFilter: string
  searchResults: OnlineSkillDto[]
  searchLoading: boolean
  managedSkills: ManagedSkill[]
  loading: boolean
  onExploreFilterChange: (value: string) => void
  onInstallSkill: (sourceUrl: string, skillName?: string) => void
  onOpenManualAdd: (tab?: 'git' | 'local') => void
  onOpenAiParse: () => void
  t: TFunction
}

function formatCount(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
  return String(n)
}

// "https://github.com/owner/repo/tree/main/x" → "owner/repo"
// "owner/repo" → "owner/repo"
function extractOwnerRepo(input: string): string {
  const stripped = input
    .replace(/^https?:\/\/github\.com\//i, '')
    .replace(/\.git$/i, '')
    .split('/tree/')[0]
    .split('/blob/')[0]
  const parts = stripped.split('/').filter(Boolean)
  if (parts.length >= 2) return `${parts[0]}/${parts[1]}`
  return stripped
}

// "https://github.com/owner/repo/tree/main/x" → "https://github.com/owner/repo"
function repoRootUrl(sourceUrl: string): string {
  const base = sourceUrl
    .split('/tree/')[0]
    .split('/blob/')[0]
    .replace(/\.git$/i, '')
  // if it doesn't start with http, treat as owner/repo
  if (!/^https?:\/\//i.test(base)) {
    return `https://github.com/${base}`
  }
  return base
}

type CollectionItem =
  | {
      kind: 'featured'
      key: string
      name: string
      summary: string
      stars: number
      source_url: string
      slug: string
    }
  | {
      kind: 'online'
      key: string
      name: string
      installs: number
      source_url: string
      source: string
    }

type ExploreCollection = {
  key: string // owner/repo
  displayName: string // owner/repo
  repoUrl: string // https://github.com/owner/repo
  items: CollectionItem[]
  totalStars: number
  totalInstalls: number
}

function groupByRepo(
  featured: FeaturedSkillDto[],
  online: OnlineSkillDto[],
): ExploreCollection[] {
  const map = new Map<string, ExploreCollection>()
  for (const s of featured) {
    const key = extractOwnerRepo(s.source_url)
    const url = repoRootUrl(s.source_url)
    const bucket =
      map.get(key) ??
      ({
        key,
        displayName: key,
        repoUrl: url,
        items: [],
        totalStars: 0,
        totalInstalls: 0,
      } as ExploreCollection)
    bucket.items.push({
      kind: 'featured',
      key: `f:${s.slug}`,
      name: s.name,
      summary: s.summary,
      stars: s.stars,
      source_url: s.source_url,
      slug: s.slug,
    })
    bucket.totalStars += s.stars
    map.set(key, bucket)
  }
  for (const s of online) {
    const key = extractOwnerRepo(s.source || s.source_url)
    const url = repoRootUrl(s.source_url)
    const bucket =
      map.get(key) ??
      ({
        key,
        displayName: key,
        repoUrl: url,
        items: [],
        totalStars: 0,
        totalInstalls: 0,
      } as ExploreCollection)
    // De-dup within this bucket: online result whose name already appears in featured is skipped.
    const existsInFeatured = bucket.items.some(
      (it) => it.kind === 'featured' && it.name.toLowerCase() === s.name.toLowerCase(),
    )
    if (!existsInFeatured) {
      bucket.items.push({
        kind: 'online',
        key: `o:${s.source}:${s.name}`,
        name: s.name,
        installs: s.installs,
        source_url: s.source_url,
        source: s.source,
      })
      bucket.totalInstalls += s.installs
    }
    map.set(key, bucket)
  }
  // Sort: collections with more skills first, then higher stars, then alphabetical.
  return Array.from(map.values()).sort((a, b) => {
    if (b.items.length !== a.items.length) return b.items.length - a.items.length
    if (b.totalStars !== a.totalStars) return b.totalStars - a.totalStars
    return a.displayName.localeCompare(b.displayName)
  })
}

const ExplorePage = ({
  featuredSkills,
  featuredLoading,
  exploreFilter,
  searchResults,
  searchLoading,
  managedSkills,
  loading,
  onExploreFilterChange,
  onInstallSkill,
  onOpenManualAdd,
  onOpenAiParse,
  t,
}: ExplorePageProps) => {
  const [expandedCollections, setExpandedCollections] = useState<Set<string>>(new Set())

  const filteredFeatured = useMemo(() => {
    if (!exploreFilter.trim()) return featuredSkills
    const lower = exploreFilter.toLowerCase()
    return featuredSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(lower) ||
        s.summary.toLowerCase().includes(lower),
    )
  }, [featuredSkills, exploreFilter])

  const collections = useMemo(
    () => groupByRepo(filteredFeatured, searchResults),
    [filteredFeatured, searchResults],
  )

  const isSearchActive = exploreFilter.trim().length >= 2

  // Track "installed" state — a skill is installed if we already have (name, owner/repo) matching.
  const installedKeys = useMemo(() => {
    const set = new Set<string>()
    for (const skill of managedSkills) {
      const src = extractOwnerRepo(skill.source_ref ?? '')
      set.add(`${skill.name.toLowerCase()}|${src.toLowerCase()}`)
    }
    return set
  }, [managedSkills])

  // A collection is fully installed if every item in it is installed.
  const isSkillInstalled = (skillName: string, sourceUrl: string) => {
    const key = `${skillName.toLowerCase()}|${extractOwnerRepo(sourceUrl).toLowerCase()}`
    return installedKeys.has(key)
  }
  const collectionInstalledStatus = (col: ExploreCollection) => {
    if (col.items.length === 0) return 'none' as const
    let installed = 0
    for (const it of col.items) if (isSkillInstalled(it.name, it.source_url)) installed++
    if (installed === 0) return 'none' as const
    if (installed === col.items.length) return 'all' as const
    return 'partial' as const
  }

  const toggleCollection = (key: string) => {
    setExpandedCollections((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  return (
    <div className="explore-page">
      <div className="explore-tabs" role="tablist" aria-label={t('addSkills')}>
        <button className="active" type="button" role="tab" aria-selected="true">
          {t('exploreTabs.online')}
        </button>
        <button type="button" role="tab" aria-selected="false" onClick={() => onOpenManualAdd('git')}>
          {t('exploreTabs.git')}
        </button>
        <button type="button" role="tab" aria-selected="false" onClick={() => onOpenManualAdd('local')}>
          {t('exploreTabs.local')}
        </button>
      </div>
      <div className="explore-hero">
        <div className="explore-search-row">
          <div className="explore-search-wrap">
            <Search size={16} className="explore-search-icon" />
            <input
              className="explore-search-input"
              placeholder={t('exploreFilterPlaceholder')}
              value={exploreFilter}
              onChange={(e) => onExploreFilterChange(e.target.value)}
            />
          </div>
          <button
            className="btn btn-secondary explore-manual-btn"
            type="button"
            onClick={onOpenAiParse}
            disabled={loading}
          >
            <Bot size={15} />
            {t('aiParse.open')}
          </button>
          <button
            className="btn btn-secondary explore-manual-btn"
            type="button"
            onClick={() => onOpenManualAdd('git')}
            disabled={loading}
          >
            <Plus size={15} />
            {t('manualAdd')}
          </button>
        </div>
        <div className="explore-source-label">{t('exploreSourceHint')}</div>
      </div>

      <div className="explore-scroll">
        {featuredLoading ? (
          <div className="explore-loading">{t('exploreLoading')}</div>
        ) : (
          <>
            {isSearchActive && searchLoading ? (
              <div className="explore-loading">{t('searchLoading')}</div>
            ) : null}

            {collections.length === 0 ? (
              <div className="explore-empty">
                {isSearchActive ? t('searchEmpty') : t('exploreEmpty')}
              </div>
            ) : (
              <div className="explore-collections">
                {collections.map((col) => {
                  const expanded = expandedCollections.has(col.key)
                  const status = collectionInstalledStatus(col)
                  return (
                    <div
                      key={col.key}
                      className={`explore-collection-card${expanded ? ' expanded' : ''}`}
                    >
                      <div
                        className="explore-collection-header"
                        role="button"
                        tabIndex={0}
                        onClick={() => toggleCollection(col.key)}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault()
                            toggleCollection(col.key)
                          }
                        }}
                      >
                        <div className="explore-collection-head-left">
                          <span
                            className="explore-collection-chevron"
                            aria-hidden="true"
                          >
                            {expanded ? (
                              <ChevronDown size={16} />
                            ) : (
                              <ChevronRight size={16} />
                            )}
                          </span>
                          <span
                            className="explore-collection-icon"
                            aria-hidden="true"
                          >
                            <FolderKanban size={16} />
                          </span>
                          <div className="explore-collection-info">
                            <div className="explore-collection-name">
                              {col.displayName}
                            </div>
                            <div className="explore-collection-meta">
                              <span>
                                {t('collectionSkillCount', { count: col.items.length })}
                              </span>
                              {col.totalStars > 0 ? (
                                <span className="explore-collection-stat">
                                  <Star size={11} />
                                  {formatCount(col.totalStars)}
                                </span>
                              ) : null}
                              {col.totalInstalls > 0 ? (
                                <span className="explore-collection-stat">
                                  {formatCount(col.totalInstalls)}{' '}
                                  {t('exploreInstallsSuffix')}
                                </span>
                              ) : null}
                            </div>
                          </div>
                        </div>
                        <div
                          className="explore-collection-head-right"
                          onClick={(e) => e.stopPropagation()}
                        >
                          {status === 'all' ? (
                            <span className="explore-btn-installed">
                              {t('status.installed')}
                            </span>
                          ) : (
                            <button
                              className="explore-btn-install"
                              type="button"
                              disabled={loading}
                              onClick={() => onInstallSkill(col.repoUrl)}
                              title={t('exploreInstallCollectionHint')}
                            >
                              {status === 'partial'
                                ? t('exploreInstallCollectionMissing')
                                : t('exploreInstallCollection')}
                            </button>
                          )}
                        </div>
                      </div>

                      {expanded ? (
                        <div className="explore-collection-body">
                          {col.items.map((item) => {
                            const installed = isSkillInstalled(item.name, item.source_url)
                            return (
                              <div key={item.key} className="explore-mini-card">
                                <div className="explore-mini-info">
                                  <div className="explore-mini-name">{item.name}</div>
                                  {item.kind === 'featured' ? (
                                    <div className="explore-mini-desc">{item.summary}</div>
                                  ) : null}
                                  <div className="explore-mini-stats">
                                    {item.kind === 'featured' ? (
                                      <span className="explore-stat">
                                        <Star size={11} />
                                        {formatCount(item.stars)}
                                      </span>
                                    ) : (
                                      <span className="explore-stat">
                                        {formatCount(item.installs)}{' '}
                                        {t('exploreInstallsSuffix')}
                                      </span>
                                    )}
                                  </div>
                                </div>
                                {installed ? (
                                  <span className="explore-btn-installed">
                                    {t('status.installed')}
                                  </span>
                                ) : (
                                  <button
                                    className="explore-btn-install"
                                    type="button"
                                    disabled={loading}
                                    onClick={() =>
                                      onInstallSkill(
                                        item.source_url,
                                        item.kind === 'online' ? item.name : undefined,
                                      )
                                    }
                                  >
                                    {t('install')}
                                  </button>
                                )}
                              </div>
                            )
                          })}
                        </div>
                      ) : null}
                    </div>
                  )
                })}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}

export default memo(ExplorePage)
