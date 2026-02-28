import { useEffect } from 'react'

export default function SlidePanel({ title, onClose, children }) {
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <div className="slide-overlay">
      <div className="slide-backdrop" onClick={onClose} />
      <div className="slide-panel">
        <div className="slide-header">
          <span className="slide-title">{title}</span>
          <button className="slide-close" onClick={onClose}>✕</button>
        </div>
        <div className="slide-body">
          {children}
        </div>
      </div>
    </div>
  )
}
