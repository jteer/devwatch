import { useEffect, useRef, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { GithubNotification, PullRequest, ConnectionStatus } from '../types'

export function useDaemon() {
  const [prs,     setPrs]     = useState<PullRequest[]>([])
  const [status,  setStatus]  = useState<ConnectionStatus>('connecting')
  const [unread,  setUnread]  = useState(0)
  const [polling, setPolling] = useState(false)
  const [events,  setEvents]  = useState<GithubNotification[]>([])
  const [settled, setSettled] = useState(false)
  const statusRef  = useRef<ConnectionStatus>('connecting')
  const settledRef = useRef(false)

  const updateStatus = useCallback((s: ConnectionStatus) => {
    statusRef.current = s
    setStatus(s)
  }, [])

  const updateNotification = useCallback((id: string, patch: Partial<GithubNotification>) => {
    setEvents(prev => prev.map(n => n.id === id ? { ...n, ...patch } : n))
  }, [])

  const markAllNotificationsSeen = useCallback(() => {
    setEvents(prev => prev.map(n => n.hidden ? n : { ...n, seen: true }))
  }, [])

  // Load persisted notifications from DB on mount
  useEffect(() => {
    invoke<GithubNotification[]>('list_notifications')
      .then(notifications => setEvents(notifications))
      .catch(console.error)
  }, [])

  useEffect(() => {
    const cleanup: Array<() => void> = []

    const listenersReady = Promise.all([
      listen<PullRequest[]>('pr-snapshot', e => setPrs(e.payload))
        .then(u => cleanup.push(u)),
      listen<string>('connection-status', e => updateStatus(e.payload as ConnectionStatus))
        .then(u => cleanup.push(u)),
      listen<number>('unread-count', e => setUnread(e.payload))
        .then(u => cleanup.push(u)),
      listen<boolean>('polling', e => {
        setPolling(e.payload)
        if (!e.payload && !settledRef.current) {
          settledRef.current = true
          setSettled(true)
        }
      }).then(u => cleanup.push(u)),
      listen<{ Notification?: GithubNotification } | null>('pr-event', e => {
        const n = e.payload?.Notification
        if (!n) return
        // Merge into existing list: update if present, prepend if new
        setEvents(prev => {
          const idx = prev.findIndex(x => x.id === n.id)
          if (idx !== -1) {
            // Preserve user's seen/hidden state — only update metadata
            const existing = prev[idx]
            const updated = { ...n, seen: existing.seen, hidden: existing.hidden }
            return [updated, ...prev.filter((_, i) => i !== idx)]
          }
          return [n, ...prev]
        })
      }).then(u => cleanup.push(u)),
    ])

    listenersReady.then(() => {
      invoke<string>('get_connection_status')
        .then(s => updateStatus(s as ConnectionStatus))
        .catch(console.error)
      invoke<PullRequest[]>('list_prs').then(setPrs).catch(console.error)
      invoke<number>('get_unread_count').then(setUnread).catch(console.error)
    })

    const poll = setInterval(async () => {
      if (statusRef.current === 'connected') {
        clearInterval(poll)
        return
      }
      const s = await invoke<string>('get_connection_status').catch(() => null)
      if (s) updateStatus(s as ConnectionStatus)
      if (s === 'connected') {
        invoke<PullRequest[]>('list_prs').then(setPrs).catch(console.error)
        clearInterval(poll)
      }
    }, 1500)

    cleanup.push(() => clearInterval(poll))

    return () => cleanup.forEach(u => u())
  }, [updateStatus])

  const openPr = useCallback((url: string) => {
    invoke('open_pr', { url }).catch(console.error)
  }, [])

  const markAllRead = useCallback(() => {
    invoke('mark_all_read').then(() => setUnread(0)).catch(console.error)
  }, [])

  return { prs, status, unread, polling, openPr, markAllRead, events, settled,
           updateNotification, markAllNotificationsSeen }
}
