import { useState } from 'react'
import { Search, BellOff, Plus } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { PrTable } from '@/components/PrTable'
import { StatusBar } from '@/components/StatusBar'
import type { PullRequest } from '@/types'
import type { useDaemon } from '@/hooks/useDaemon'

interface DashboardProps {
  daemon:      ReturnType<typeof useDaemon>
  isDemo?:     boolean
  onAddDemo?:  () => void
}

export default function Dashboard({ daemon, isDemo, onAddDemo }: DashboardProps) {
  const [filter, setFilter] = useState('')
  const { prs, status, unread, polling, openPr, markAllRead } = daemon

  function handleRowClick(pr: PullRequest) {
    if (!isDemo && pr.url) openPr(pr.url)
  }

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
        <span className="text-sm text-muted-foreground ml-1">
          {prs.length} PR{prs.length !== 1 ? 's' : ''}
        </span>
        <div className="ml-auto flex items-center gap-1">
          {onAddDemo && (
            <Button variant="outline" size="sm" onClick={onAddDemo} className="gap-1.5">
              <Plus className="h-4 w-4" />
              Add demo PR
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

      {/* Table — scrollable */}
      <div className="flex-1 overflow-auto">
        <PrTable prs={prs} filter={filter} onRowClick={handleRowClick} />
      </div>

      <StatusBar status={status} polling={polling} unread={unread} />
    </div>
  )
}
