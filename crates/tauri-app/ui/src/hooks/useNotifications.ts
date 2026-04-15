import { useEffect, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import { toast } from 'sonner'
import type { GithubNotification, NotificationMode, PullRequest } from '../types'

// Serde externally-tagged enum shape from Rust
type VcsEvent =
  | { NewPullRequest: PullRequest }
  | { PullRequestUpdated: { old: PullRequest; new: PullRequest } }
  | { PullRequestClosed: PullRequest }
  | { Notification: GithubNotification }

export function reasonLabel(reason: string): string {
  switch (reason) {
    case 'comment':          return 'New comment'
    case 'mention':          return 'You were mentioned'
    case 'review_requested': return 'Review requested'
    case 'ci_activity':      return 'CI update'
    case 'assign':           return 'You were assigned'
    case 'author':           return 'Activity on your item'
    default:                 return 'GitHub notification'
  }
}

// Cached permission state so we don't call requestPermission() on every event
let osPermissionGranted: boolean | null = null

export async function ensureOsPermission(): Promise<boolean> {
  if (osPermissionGranted !== null) return osPermissionGranted
  try {
    const { isPermissionGranted, requestPermission } =
      await import('@tauri-apps/plugin-notification')
    let granted = await isPermissionGranted()
    console.log('[devwatch] notification permission granted:', granted)
    if (!granted) {
      const result = await requestPermission()
      console.log('[devwatch] requestPermission result:', result)
      granted = result === 'granted'
    }
    osPermissionGranted = granted
    return granted
  } catch (e) {
    console.error('[devwatch] OS notification permission error:', e)
    return false
  }
}

export async function sendOsNotification(title: string, body: string) {
  const granted = await ensureOsPermission()
  if (!granted) {
    console.warn('[devwatch] OS notification skipped — permission not granted')
    return
  }
  try {
    const { sendNotification } = await import('@tauri-apps/plugin-notification')
    sendNotification({ title, body })
  } catch (e) {
    console.error('[devwatch] sendNotification error:', e)
  }
}

export function notify(mode: NotificationMode, title: string, body: string) {
  if (mode === 'in_app' || mode === 'both') {
    toast(title, { description: body })
  }
  if (mode === 'os' || mode === 'both') {
    sendOsNotification(title, body)
  }
}

export function useNotifications(mode: NotificationMode, isDemo: boolean, settled = false) {
  // Track settled via ref so the listener closure doesn't go stale on each re-render
  const settledRef = useRef(settled)
  useEffect(() => { settledRef.current = settled }, [settled])
  // Eagerly request OS permission as soon as the mode requires it,
  // so the macOS dialog appears before the first event fires.
  useEffect(() => {
    if (mode === 'os' || mode === 'both') {
      ensureOsPermission()
    }
  }, [mode])

  useEffect(() => {
    if (mode === 'off' || isDemo) return

    let unlisten: (() => void) | null = null

    listen<VcsEvent | null>('pr-event', e => {
      // Suppress toasts during the startup catch-up flood; PR events always go through
      const event = e.payload
      if (!event) return
      const isNotification = 'Notification' in event
      if (isNotification && !settledRef.current) return

      if ('NewPullRequest' in event) {
        const pr = event.NewPullRequest
        notify(mode, 'New pull request', `${pr.repo} — ${pr.title} by ${pr.author}`)
      } else if ('PullRequestUpdated' in event) {
        const pr = event.PullRequestUpdated.new
        notify(mode, 'PR updated', `${pr.repo} — ${pr.title}`)
      } else if ('Notification' in event) {
        const n = event.Notification
        notify(mode, reasonLabel(n.reason), `${n.repo} — ${n.subject_title}`)
      }
    }).then(u => { unlisten = u })

    return () => { unlisten?.() }
  }, [mode, isDemo])
}
