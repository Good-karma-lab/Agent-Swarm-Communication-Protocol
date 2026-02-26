import { useEffect } from 'react'

export function usePolling(callback, intervalMs = 5000) {
  useEffect(() => {
    callback()
    const t = setInterval(callback, intervalMs)
    return () => clearInterval(t)
  }, [callback, intervalMs])
}
