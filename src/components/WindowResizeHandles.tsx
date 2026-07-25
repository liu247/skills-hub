import { memo, type PointerEvent } from 'react'

type ResizeDirection = 'East' | 'South' | 'SouthEast'

type WindowResizeHandlesProps = {
  enabled: boolean
}

const WindowResizeHandles = ({ enabled }: WindowResizeHandlesProps) => {
  if (!enabled) return null

  const startResize = (direction: ResizeDirection) => (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return
    event.preventDefault()
    void import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => getCurrentWindow().startResizeDragging(direction))
      .catch(() => undefined)
  }

  return (
    <>
      <div
        className="window-resize-handle east"
        aria-hidden="true"
        onPointerDown={startResize('East')}
      />
      <div
        className="window-resize-handle south"
        aria-hidden="true"
        onPointerDown={startResize('South')}
      />
      <div
        className="window-resize-handle south-east"
        aria-hidden="true"
        onPointerDown={startResize('SouthEast')}
      />
    </>
  )
}

export default memo(WindowResizeHandles)
