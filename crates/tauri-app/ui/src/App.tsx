import { useState, useMemo, useCallback } from 'react'
import { LayoutDashboard, Settings as SettingsIcon, FlaskConical } from 'lucide-react'
import { Toaster } from 'sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { Button } from '@/components/ui/button'
import { ThemeToggle } from '@/components/ThemeToggle'
import Dashboard from '@/pages/Dashboard'
import Settings from '@/pages/Settings'
import { useDaemon } from '@/hooks/useDaemon'
import { useDemoData } from '@/hooks/useDemoData'
import { useNotifications, notify } from '@/hooks/useNotifications'
import type { NotificationMode } from '@/types'

type Page = 'dashboard' | 'settings'

export default function App() {
  const [page, setPage]                   = useState<Page>('dashboard')
  const [isDemo, setIsDemo]               = useState(false)
  const [notifMode, setNotifMode]         = useState<NotificationMode>('in_app')
  const daemon   = useDaemon()
  const demoData = useDemoData()
  useNotifications(notifMode, isDemo)

  // Fire a notification when a demo PR is manually added
  const handleAddDemo = useCallback(() => {
    const pr = demoData.addPr()
    notify(notifMode, 'New pull request', `${pr.repo} — ${pr.title} by ${pr.author}`)
  }, [demoData, notifMode])

  // When demo is active, swap out prs and silence openPr
  const noOpOpen = useCallback(() => {}, [])
  const activeDaemon = useMemo(() => {
    if (!isDemo) return daemon
    return { ...daemon, prs: demoData.prs, openPr: noOpOpen }
  }, [isDemo, daemon, demoData.prs, noOpOpen])

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
              variant={page === 'dashboard' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setPage('dashboard')}
            >
              <LayoutDashboard className="h-4 w-4 mr-1.5" />
              Dashboard
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
          {page === 'dashboard'
            ? <Dashboard
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
