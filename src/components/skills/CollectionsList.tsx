import { memo } from 'react'
import { FolderKanban, HelpCircle, Pencil, Trash2 } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { CollectionDto } from './types'

type CollectionsListProps = {
  collections: CollectionDto[]
  uncategorizedCount: number
  formatRelative: (ms: number | null | undefined) => string
  onOpenCollection: (name: string | null) => void
  onRenameCollection: (oldName: string, newName: string) => void
  onClearCollection: (name: string) => void
  t: TFunction
}

const CollectionsList = ({
  collections,
  uncategorizedCount,
  formatRelative,
  onOpenCollection,
  onRenameCollection,
  onClearCollection,
  t,
}: CollectionsListProps) => {
  const isEmpty = collections.length === 0 && uncategorizedCount === 0
  return (
    <div className="collections-list">
      {isEmpty ? (
        <div className="empty">{t('collectionsEmpty')}</div>
      ) : (
        <div className="collections-grid">
          {collections.map((collection) => (
            <div
              className="collection-card"
              key={collection.name}
              role="button"
              tabIndex={0}
              onClick={() => onOpenCollection(collection.name)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onOpenCollection(collection.name)
                }
              }}
            >
              <div className="collection-card-head">
                <span className="collection-card-icon" aria-hidden="true">
                  <FolderKanban size={18} />
                </span>
                <span className="collection-card-name">{collection.name}</span>
              </div>
              <div className="collection-card-meta">
                <span>{t('collectionSkillCount', { count: collection.skill_count })}</span>
                <span>{formatRelative(collection.updated_at)}</span>
              </div>
              <div className="collection-card-actions">
                <button
                  type="button"
                  className="collection-card-action"
                  title={t('renameCollection')}
                  aria-label={t('renameCollection')}
                  onClick={(event) => {
                    event.stopPropagation()
                    const next = window.prompt(
                      t('renameCollectionPrompt'),
                      collection.name,
                    )
                    if (next?.trim() && next.trim() !== collection.name) {
                      onRenameCollection(collection.name, next.trim())
                    }
                  }}
                >
                  <Pencil size={14} />
                </button>
                <button
                  type="button"
                  className="collection-card-action danger"
                  title={t('clearCollection')}
                  aria-label={t('clearCollection')}
                  onClick={(event) => {
                    event.stopPropagation()
                    if (window.confirm(t('clearCollectionConfirm', { name: collection.name }))) {
                      onClearCollection(collection.name)
                    }
                  }}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))}
          {uncategorizedCount > 0 ? (
            <div
              className="collection-card uncategorized"
              role="button"
              tabIndex={0}
              onClick={() => onOpenCollection(null)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onOpenCollection(null)
                }
              }}
            >
              <div className="collection-card-head">
                <span className="collection-card-icon" aria-hidden="true">
                  <HelpCircle size={18} />
                </span>
                <span className="collection-card-name">{t('uncategorizedCollection')}</span>
              </div>
              <div className="collection-card-meta">
                <span>{t('collectionSkillCount', { count: uncategorizedCount })}</span>
                <span>{t('uncategorizedHint')}</span>
              </div>
            </div>
          ) : null}
        </div>
      )}
    </div>
  )
}

export default memo(CollectionsList)
