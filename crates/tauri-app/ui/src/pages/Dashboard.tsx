import { useMemo, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Search, BellOff, Plus, Eye } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { FeedTable, type FeedItem } from '@/components/PrTable'
import { StatusBar } from '@/components/StatusBar'
import type { useDaemon } from '@/hooks/useDaemon'

interface DashboardProps {
  daemon:      ReturnType<typeof useDaemon>
  isDemo?:     boolean
  onAddDemo?:  () => void
}

type QuickFilter = 'all' | 'pr' | 'issue' | 'comment' | 'mention' | 'ci'

const QUICK_FILTERS: { id: QuickFilter; label: string }[] = [
  { id: 'all',     label: 'All'      },
  { id: 'pr',      label: 'PRs'      },
  { id: 'issue',   label: 'Issues'   },
  { id: 'comment', label: 'Comments' },
  { id: 'mention', label: 'Mentions' },
  { id: 'ci',      label: 'CI'       },
]

export default function Dashboard({ daemon, isDemo, onAddDemo }: DashboardProps) {
  const [filter,       setFilter]       = useState('')
  const [quickFilter,  setQuickFilter]  = useState<QuickFilter>('all')
  const [showHidden,   setShowHidden]   = useState(false)
  const { prs, events, status, unread, polling, openPr,
          markAllRead: markAllReadDaemon, updateNotification, markAllNotificationsSeen } = daemon

  const seenIds = useMemo(
    () => new Set(events.filter(n => n.seen).map(n => n.id)),
    [events]
  )

  const feedItems = useMemo((): FeedItem[] => {
    const visibleEvents = showHidden
      ? events
      : events.filter(n => !n.hidden)

    const items: FeedItem[] = [
      ...prs.map(pr => ({ kind: 'pr' as const, data: pr })),
      ...visibleEvents.map(n => ({ kind: 'notification' as const, data: n })),
    ]
    items.sort((a, b) => {
      const ta = a.kind === 'pr' ? a.data.created_at : a.data.updated_at
      const tb = b.kind === 'pr' ? b.data.created_at : b.data.updated_at
      return tb - ta
    })

    if (quickFilter === 'all') return items
    return items.filter(item => {
      if (item.kind === 'pr') return quickFilter === 'pr'
      const { reason, subject_type } = item.data
      if (quickFilter === 'pr')      return subject_type === 'PullRequest'
      if (quickFilter === 'issue')   return subject_type === 'Issue'
      if (quickFilter === 'comment') return reason === 'comment'
      if (quickFilter === 'mention') return reason === 'mention'
      if (quickFilter === 'ci')      return reason === 'ci_activity' || subject_type === 'CheckSuite' || subject_type === 'WorkflowRun'
      return true
    })
  }, [prs, events, showHidden, quickFilter])

  const toggleSeen = useCallback((id: string) => {
    const n = events.find(e => e.id === id)
    if (!n) return
    const seen = !n.seen
    updateNotification(id, { seen })
    invoke('mark_notification_seen', { id, seen }).catch(console.error)
  }, [events, updateNotification])

  const hideNotification = useCallback((id: string) => {
    updateNotification(id, { hidden: true, seen: true })
    invoke('hide_notification', { id }).catch(console.error)
  }, [updateNotification])

  const unhideNotification = useCallback((id: string) => {
    updateNotification(id, { hidden: false })
    invoke('unhide_notification', { id }).catch(console.error)
  }, [updateNotification])

  const markAllRead = useCallback(() => {
    markAllReadDaemon()
    markAllNotificationsSeen()
    invoke('mark_all_notifications_seen').catch(console.error)
  }, [markAllReadDaemon, markAllNotificationsSeen])

  function handleRowClick(item: FeedItem) {
    if (isDemo) return
    if (item.data.url) openPr(item.data.url)
  }

  const prCount        = prs.length
  const visibleEvents  = events.filter(n => !n.hidden)
  const hiddenCount    = events.filter(n => n.hidden).length
  const eventCount     = visibleEvents.length

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b shrink-0">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground pointer-events-none" />
          <Input
            placeholder="Filter by title, author, repo…"
            value={filter}
            onChange={e => setFilter(e.target.value)}
            className="pl-8"
          />
        </div>
        <div className="flex items-center gap-1">
          {QUICK_FILTERS.map(({ id, label }) => (
            <Button
              key={id}
              variant={quickFilter === id ? 'secondary' : 'ghost'}
              size="sm"
              className="h-8 px-2.5 text-xs"
              onClick={() => setQuickFilter(id)}
            >
              {label}
            </Button>
          ))}
        </div>
        <span className="text-sm text-muted-foreground ml-1">
          {prCount} PR{prCount !== 1 ? 's' : ''}
          {eventCount > 0 && (
            <span className="ml-1 text-muted-foreground/60">
              · {eventCount} notification{eventCount !== 1 ? 's' : ''}
            </span>
          )}
        </span>
        <div className="ml-auto flex items-center gap-1">
          {onAddDemo && (
            <Button variant="outline" size="sm" onClick={onAddDemo} className="gap-1.5">
              <Plus className="h-4 w-4" />
              Add demo event
            </Button>
          )}
          {hiddenCount > 0 && (
            <Button
              variant={showHidden ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setShowHidden(v => !v)}
              className="gap-1.5"
            >
              <Eye className="h-4 w-4" />
              {showHidden ? 'Hide hidden' : `Show hidden (${hiddenCount})`}
            </Button>
          )}
          {!isDemo && unread > 0 && (
            <Button variant="ghost" size="sm" onClick={markAllRead} className="gap-1.5">
              <BellOff className="h-4 w-4" />
              Mark all read
            </Button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        <FeedTable
          items={feedItems}
          filter={filter}
          onRowClick={handleRowClick}
          seenIds={seenIds}
          onToggleSeen={toggleSeen}
          onHide={hideNotification}
          onUnhide={unhideNotification}
        />
      </div>

      <StatusBar status={status} polling={polling} unread={unread} />
    </div>
  )
}
