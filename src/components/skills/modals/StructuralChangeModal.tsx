import { memo } from 'react'
import { AlertTriangle } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { StructuralChangeReport } from '../types'

type StructuralChangeModalProps = {
  open: boolean
  loading: boolean
  skillName: string
  report: StructuralChangeReport | null
  onCancel: () => void
  onReinstall: () => void
  onForceUpdate: () => void
  t: TFunction
}

const StructuralChangeModal = ({
  open,
  loading,
  skillName,
  report,
  onCancel,
  onReinstall,
  onForceUpdate,
  t,
}: StructuralChangeModalProps) => {
  if (!open || !report) return null

  return (
    <div className="modal-backdrop" onClick={loading ? undefined : onCancel}>
      <div
        className="modal modal-structural-change"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div className="modal-title">
            <span className="modal-title-icon warning" aria-hidden="true">
              <AlertTriangle size={18} />
            </span>
            {t('structuralChange.title', { name: skillName })}
          </div>
          <div className="modal-subtitle">{report.summary}</div>
        </div>
        <div className="modal-body">
          <p className="structural-change-body">{t('structuralChange.description')}</p>

          {report.removed_paths.length > 0 || report.added_paths.length > 0 ? (
            <div className="structural-change-diff">
              {report.removed_paths.length > 0 ? (
                <div>
                  <div className="structural-change-diff-title">
                    {t('structuralChange.removedPaths')}
                  </div>
                  <ul>
                    {report.removed_paths.map((p) => (
                      <li key={p} className="structural-change-removed">
                        {p}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
              {report.added_paths.length > 0 ? (
                <div>
                  <div className="structural-change-diff-title">
                    {t('structuralChange.addedPaths')}
                  </div>
                  <ul>
                    {report.added_paths.map((p) => (
                      <li key={p} className="structural-change-added">
                        {p}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </div>
          ) : null}

          {report.suggested_new_subpath ? (
            <div className="structural-change-suggestion">
              <strong>{t('structuralChange.suggestedNewSubpath')}: </strong>
              <code>{report.suggested_new_subpath}</code>
            </div>
          ) : null}
        </div>

        <div className="modal-footer structural-change-actions">
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onCancel}
            disabled={loading}
          >
            {t('cancel')}
          </button>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onForceUpdate}
            disabled={loading}
            title={t('structuralChange.forceUpdateHint')}
          >
            {t('structuralChange.forceUpdate')}
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={onReinstall}
            disabled={loading}
            title={t('structuralChange.reinstallHint')}
          >
            {t('structuralChange.reinstall')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(StructuralChangeModal)
