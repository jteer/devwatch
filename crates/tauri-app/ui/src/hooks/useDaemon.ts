import { useEffect, useRef, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { PullRequest, ConnectionStatus } from '../types'

export function useDaemon() {
  const [prs,     setPrs]     = useState<PullRequest[]>([])
  const [status,  setStatus]  = useState<ConnectionStatus>('connecting')
  const [unread,  setUnread]  = useState(0)
  const [polling, setPolling] = useState(false)
  const statusRef = useRef<ConnectionStatus>('connecting')

  const updateStatus = useCallback((s: ConnectionStatus) => {
    statusRef.current = s
    setStatus(s)
  }, [])

  useEffect(() => {
    const cleanup: Array<() => void> = []

    // Register event listeners first, then seed state, so we don't miss
    // events that fire between seed and listener registration.
    const listenersReady = Promise.all([
      listen<PullRequest[]>('pr-snapshot', e => setPrs(e.payload))
        .then(u => cleanup.push(u)),
      listen<string>('connection-status', e => updateStatus(e.payload as ConnectionStatus))
        .then(u => cleanup.push(u)),
      listen<number>('unread-count', e => setUnread(e.payload))
        .then(u => cleanup.push(u)),
      listen<boolean>('polling', e => setPolling(e.payload))
        .then(u => cleanup.push(u)),
    ])

    // Once listeners are registered, seed current state from Rust.
    listenersReady.then(() => {
      invoke<string>('get_connection_status')
        .then(s => updateStatus(s as ConnectionStatus))
        .catch(console.error)
      invoke<PullRequest[]>('list_prs').then(setPrs).catch(console.error)
      invoke<number>('get_unread_count').then(setUnread).catch(console.error)
    })

    // Poll connection status every 1.5 s until connected.
    // Guards against events that fire in the brief window before listeners
    // finish registering on the Tauri side.
    const poll = setInterval(async () => {
      if (statusRef.current === 'connected') {
        clearInterval(poll)
        return
      }
      const s = await invoke<string>('get_connection_status').catch(() => null)
      if (s) updateStatus(s as ConnectionStatus)
      // Also refresh PRs on first successful connection.
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
    invoke('mark_all_read')
      .then(() => setUnread(0))
      .catch(console.error)
  }, [])

  return { prs, status, unread, polling, openPr, markAllRead }
}
