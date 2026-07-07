import { memo, useMemo, useState } from 'react'
import { ArrowLeft, Plus, Search, Tag } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { TagWithCountDto } from './types'

type TagsPageProps = {
  tags: TagWithCountDto[]
  untaggedCount: number
  loading: boolean
  formatRelative: (ms: number | null | undefined) => string
  embedded?: boolean
  onBack?: () => void
  onReviewUntagged: () => void
  onViewTag: (tagId: number) => void
  onCreateTag: (name: string) => void
  onRenameTag: (tagId: number, name: string) => void
  onDeleteTag: (tag: TagWithCountDto) => void
  t: TFunction
}

const TagsPage = ({
  tags,
  untaggedCount,
  loading,
  formatRelative,
  embedded = false,
  onBack,
  onReviewUntagged,
  onViewTag,
  onCreateTag,
  onRenameTag,
  onDeleteTag,
  t,
}: TagsPageProps) => {
  const [query, setQuery] = useState('')
  const [newTagName, setNewTagName] = useState('')
  const filteredTags = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    if (!normalized) return tags
    return tags.filter((tag) => tag.name.toLowerCase().includes(normalized))
  }, [query, tags])

  const submitNewTag = () => {
    const name = newTagName.trim()
    if (!name) return
    onCreateTag(name)
    setNewTagName('')
  }

  return (
    <div className={`tags-page${embedded ? ' embedded' : ''}`}>
      {!embedded ? (
        <div className="detail-header">
          <button className="btn btn-secondary" type="button" onClick={onBack}>
            <ArrowLeft size={16} />
            {t('back')}
          </button>
          <div>
            <div className="detail-skill-name">{t('tags')}</div>
            <div className="tags-page-subtitle">{t('tagsHelp')}</div>
          </div>
        </div>
      ) : null}

      <div className="tags-review-row">
        <div className="tags-review-left">
          <Tag size={16} />
          <span>{t('untaggedSkillsCount', { count: untaggedCount })}</span>
        </div>
        <button
          className="btn btn-secondary"
          type="button"
          onClick={onReviewUntagged}
          disabled={untaggedCount === 0}
        >
          {t('review')}
        </button>
      </div>

      <div className="tags-toolbar">
        <div className="search-container tags-search">
          <Search size={16} className="search-icon-abs" />
          <input
            className="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('searchTags')}
          />
        </div>
        <div className="tags-new-row">
          <input
            className="search-input"
            value={newTagName}
            onChange={(event) => setNewTagName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') submitNewTag()
            }}
            placeholder={t('newTagPlaceholder')}
          />
          <button
            className="btn btn-primary"
            type="button"
            onClick={submitNewTag}
            disabled={loading || !newTagName.trim()}
          >
            <Plus size={14} />
            {t('newTag')}
          </button>
        </div>
      </div>

      <div className="tags-table">
        <div className="tags-table-row tags-table-head">
          <span>{t('tagName')}</span>
          <span>{t('skills')}</span>
          <span>{t('lastUsed')}</span>
          <span>{t('actionsLabel')}</span>
        </div>
        {filteredTags.length === 0 ? (
          <div className="empty">{t('tagsEmpty')}</div>
        ) : (
          filteredTags.map((tag) => (
            <div className="tags-table-row" key={tag.id}>
              <span className="tags-table-name">{tag.name}</span>
              <span>{tag.skill_count}</span>
              <span>{formatRelative(tag.updated_at)}</span>
              <span className="tags-table-actions">
                <button type="button" onClick={() => onViewTag(tag.id)}>
                  {t('view')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const nextName = window.prompt(t('renameTagPrompt'), tag.name)
                    if (nextName?.trim()) onRenameTag(tag.id, nextName)
                  }}
                >
                  {t('rename')}
                </button>
                <button type="button" onClick={() => onDeleteTag(tag)}>
                  {t('deleteAction')}
                </button>
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

export default memo(TagsPage)
