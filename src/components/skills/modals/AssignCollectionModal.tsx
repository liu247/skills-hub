import { memo, useMemo, useState } from 'react'
import type { TFunction } from 'i18next'
import type { CollectionDto } from '../types'

type AssignCollectionModalProps = {
  open: boolean
  loading: boolean
  skillCount: number
  collections: CollectionDto[]
  onCancel: () => void
  onConfirm: (collection: string | null) => void
  t: TFunction
}

const AssignCollectionModal = ({
  open,
  loading,
  skillCount,
  collections,
  onCancel,
  onConfirm,
  t,
}: AssignCollectionModalProps) => {
  // 'existing' | 'new' | 'clear'
  const [mode, setMode] = useState<'existing' | 'new' | 'clear'>(
    collections.length > 0 ? 'existing' : 'new',
  )
  const [selectedExisting, setSelectedExisting] = useState<string>(
    collections[0]?.name ?? '',
  )
  const [newName, setNewName] = useState('')

  const sortedCollections = useMemo(
    () => [...collections].sort((a, b) => a.name.localeCompare(b.name)),
    [collections],
  )

  if (!open) return null

  const canSubmit =
    mode === 'clear'
      ? true
      : mode === 'existing'
        ? selectedExisting.trim().length > 0
        : newName.trim().length > 0

  const handleSubmit = () => {
    if (!canSubmit || loading) return
    if (mode === 'clear') {
      onConfirm(null)
    } else if (mode === 'existing') {
      onConfirm(selectedExisting.trim())
    } else {
      onConfirm(newName.trim())
    }
  }

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-header">
          <div className="modal-title">{t('assignCollectionTitle')}</div>
          <div className="modal-subtitle">
            {t('assignCollectionSubtitle', { count: skillCount })}
          </div>
        </div>
        <div className="modal-body">
          <div className="assign-collection-modes">
            {sortedCollections.length > 0 ? (
              <label className="assign-collection-mode">
                <input
                  type="radio"
                  name="assign-collection-mode"
                  checked={mode === 'existing'}
                  onChange={() => setMode('existing')}
                />
                <span>{t('assignCollectionExisting')}</span>
              </label>
            ) : null}
            <label className="assign-collection-mode">
              <input
                type="radio"
                name="assign-collection-mode"
                checked={mode === 'new'}
                onChange={() => setMode('new')}
              />
              <span>{t('assignCollectionNew')}</span>
            </label>
            <label className="assign-collection-mode">
              <input
                type="radio"
                name="assign-collection-mode"
                checked={mode === 'clear'}
                onChange={() => setMode('clear')}
              />
              <span>{t('assignCollectionClear')}</span>
            </label>
          </div>

          {mode === 'existing' && sortedCollections.length > 0 ? (
            <select
              className="settings-input"
              value={selectedExisting}
              onChange={(event) => setSelectedExisting(event.target.value)}
            >
              {sortedCollections.map((c) => (
                <option key={c.name} value={c.name}>
                  {c.name} ({c.skill_count})
                </option>
              ))}
            </select>
          ) : null}

          {mode === 'new' ? (
            <input
              className="settings-input"
              value={newName}
              onChange={(event) => setNewName(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') handleSubmit()
              }}
              placeholder={t('assignCollectionNewPlaceholder')}
              autoFocus
            />
          ) : null}
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" type="button" onClick={onCancel} disabled={loading}>
            {t('cancel')}
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={handleSubmit}
            disabled={!canSubmit || loading}
          >
            {t('confirm')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(AssignCollectionModal)
