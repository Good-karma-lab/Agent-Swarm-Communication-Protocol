import { useState, useEffect } from 'react'
import { submitTask } from '../api/client.js'

export default function SubmitTaskModal({ onClose, onSubmit }) {
  const [description, setDescription] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  async function handleSubmit() {
    if (!description.trim()) return
    setLoading(true); setError('')
    try {
      await submitTask({ description: description.trim() })
      onSubmit?.()
      onClose()
    } catch (err) {
      setError(err.message || 'Submit failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-title">Submit Task</div>

        <div className="form-row">
          <label className="form-label">Task Description</label>
          <textarea
            rows={4}
            placeholder="Describe the task…"
            value={description}
            onChange={e => setDescription(e.target.value)}
            autoFocus
          />
        </div>

        {error && <div style={{color:'var(--coral)', fontSize:12, marginBottom:8}}>{error}</div>}

        <div style={{display:'flex', gap:8, justifyContent:'flex-end'}}>
          <button className="btn" onClick={onClose}>Cancel</button>
          <button
            className="btn btn-primary"
            onClick={handleSubmit}
            disabled={!description.trim() || loading}
          >
            {loading ? 'Submitting…' : 'Submit'}
          </button>
        </div>
      </div>
    </div>
  )
}
