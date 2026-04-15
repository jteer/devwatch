import { useState, useMemo, useCallback } from 'react'
import { GitPullRequest, Bell, Settings as SettingsIcon, FlaskConical } from 'lucide-react'
import { Toaster } from 'sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { Button } from '@/components/ui/button'
import { ThemeToggle } from '@/components/ThemeToggle'
import PullRequests from '@/pages/PullRequests'
import Notifications from '@/pages/Dashboard'
import Settings from '@/pages/Settings'
import { useDaemon } from '@/hooks/useDaemon'
import { useDemoData } from '@/hooks/useDemoData'
import { useNotifications, notify, reasonLabel } from '@/hooks/useNotifications'
import type { NotificationMode } from '@/types'

type Page = 'prs' | 'notifications' | 'settings'

export default function App() {
  const [page, setPage]                   = useState<Page>('prs')
  const [isDemo, setIsDemo]               = useState(false)
  const [notifMode, setNotifMode]         = useState<NotificationMode>('in_app')
  const daemon   = useDaemon()
  const demoData = useDemoData()
  useNotifications(notifMode, isDemo, daemon.settled)

  // Fire a notification when a demo event is manually added
  const handleAddDemo = useCallback(() => {
    const item = demoData.addItem()
    if (item.kind === 'pr') {
      notify(notifMode, 'New pull request', `${item.pr.repo} — ${item.pr.title} by ${item.pr.author}`)
    } else {
      const n = item.notification
      notify(notifMode, reasonLabel(n.reason), `${n.repo} — ${n.subject_title}`)
    }
  }, [demoData, notifMode])

  // When demo is active, swap out prs/events and silence openPr
  const noOpOpen = useCallback(() => {}, [])
  const activeDaemon = useMemo(() => {
    if (!isDemo) return daemon
    return { ...daemon, prs: demoData.prs, events: demoData.events, openPr: noOpOpen }
  }, [isDemo, daemon, demoData.prs, demoData.events, noOpOpen])

  return (
    <TooltipProvider>
      <div className="flex flex-col h-screen bg-background text-foreground select-none">
        {/* Nav bar */}
        <header className="flex items-center justify-between px-4 py-2 border-b shrink-0">
          <div className="flex items-center gap-3">
            <span className="font-semibold text-sm tracking-tight">devwatch</span>
            {isDemo && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold bg-amber-500/20 text-amber-500 border border-amber-500/30 uppercase tracking-wide">
                demo
              </span>
            )}
            {!isDemo && activeDaemon.unread > 0 && (
              <span className="inline-flex items-center justify-center h-4 min-w-4 px-1 rounded-full bg-primary text-primary-foreground text-[10px] font-bold">
                {activeDaemon.unread}
              </span>
            )}
          </div>
          <nav className="flex items-center gap-1">
            <Button
              variant={page === 'prs' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setPage('prs')}
            >
              <GitPullRequest className="h-4 w-4 mr-1.5" />
              Pull Requests
            </Button>
            <Button
              variant={page === 'notifications' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setPage('notifications')}
            >
              <Bell className="h-4 w-4 mr-1.5" />
              Notifications
            </Button>
            <Button
              variant={page === 'settings' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setPage('settings')}
            >
              <SettingsIcon className="h-4 w-4 mr-1.5" />
              Settings
            </Button>
            <div className="w-px h-4 bg-border mx-1" />
            <Button
              variant={isDemo ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setIsDemo(d => !d)}
              title="Toggle demo mode"
            >
              <FlaskConical className="h-4 w-4" />
            </Button>
            <ThemeToggle />
          </nav>
        </header>

        {/* Page content */}
        <main className="flex-1 overflow-hidden">
          {page === 'prs'
            ? <PullRequests
                daemon={activeDaemon}
                isDemo={isDemo}
                onAddDemo={isDemo ? handleAddDemo : undefined}
              />
            : page === 'notifications'
            ? <Notifications
                daemon={activeDaemon}
                isDemo={isDemo}
                onAddDemo={isDemo ? handleAddDemo : undefined}
              />
            : <Settings notifMode={notifMode} onNotifModeChange={setNotifMode} />
          }
        </main>
      </div>
      <Toaster richColors position="bottom-right" />
    </TooltipProvider>
  )
}
