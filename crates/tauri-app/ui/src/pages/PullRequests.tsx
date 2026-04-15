import { useMemo, useState } from 'react'
import { Github, Gitlab, GitPullRequest, Search, Users, Plus, ChevronRight } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { StatusBar } from '@/components/StatusBar'
import { formatAge, formatDate } from '@/lib/utils'
import type { PullRequest } from '@/types'
import type { useDaemon } from '@/hooks/useDaemon'

interface PullRequestsProps {
  daemon:      ReturnType<typeof useDaemon>
  isDemo?:     boolean
  onAddDemo?:  () => void
}

function StateBadge({ state }: { state: string }) {
  const variant =
    state === 'open'   ? 'success'     :
    state === 'merged' ? 'merged'      :
    'destructive'
  return <Badge variant={variant}>{state}</Badge>
}

interface PrCardProps {
  pr:      PullRequest
  onClick: (url: string) => void
}

function PrCard({ pr, onClick }: PrCardProps) {
  const ProviderIcon = pr.provider === 'gitlab' ? Gitlab : Github

  return (
    <div
      className="rounded-md border bg-card hover:bg-accent/30 cursor-pointer p-3 space-y-1.5 transition-colors"
      onClick={() => onClick(pr.url)}
    >
      {/* Header row: provider icon, PR number, badges */}
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <ProviderIcon className="h-3 w-3 shrink-0 opacity-60" />
        <span className="font-mono">#{pr.number}</span>
        <span className="flex-1" />
        {pr.draft && (
          <Badge variant="outline" className="text-[10px] py-0 px-1.5">draft</Badge>
        )}
        <StateBadge state={pr.state} />
      </div>

      {/* Title */}
      <p className="text-sm font-medium leading-snug line-clamp-2">{pr.title}</p>

      {/* Footer: author, reviewers, age */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground flex-wrap">
        <span>@{pr.author}</span>
        {pr.reviewers.length > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="flex items-center gap-1 cursor-default">
                <Users className="h-3 w-3 shrink-0" />
                {pr.reviewers.join(', ')}
              </span>
            </TooltipTrigger>
            <TooltipContent>Reviewers</TooltipContent>
          </Tooltip>
        )}
        {pr.assignees.length > 0 && pr.assignees.some(a => !pr.reviewers.includes(a)) && (
          <span className="text-muted-foreground/60">
            → {pr.assignees.filter(a => !pr.reviewers.includes(a)).join(', ')}
          </span>
        )}
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="ml-auto cursor-default">{formatAge(pr.created_at)}</span>
          </TooltipTrigger>
          <TooltipContent>{formatDate(pr.created_at)}</TooltipContent>
        </Tooltip>
      </div>
    </div>
  )
}

export default function PullRequests({ daemon, isDemo, onAddDemo }: PullRequestsProps) {
  const [filter,      setFilter]      = useState('')
  const [showAll,     setShowAll]     = useState(false)
  const [collapsed,   setCollapsed]   = useState<Set<string>>(new Set())
  const { prs, status, polling, unread, openPr } = daemon

  const filtered = useMemo(() => {
    let list = showAll ? prs : prs.filter(pr => pr.state === 'open')
    if (filter) {
      const q = filter.toLowerCase()
      list = list.filter(pr =>
        pr.title.toLowerCase().includes(q) ||
        pr.author.toLowerCase().includes(q) ||
        pr.repo.toLowerCase().includes(q)
      )
    }
    return list.sort((a, b) => b.created_at - a.created_at)
  }, [prs, showAll, filter])

  const byRepo = useMemo(() => {
    const map = new Map<string, PullRequest[]>()
    for (const pr of filtered) {
      if (!map.has(pr.repo)) map.set(pr.repo, [])
      map.get(pr.repo)!.push(pr)
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b))
  }, [filtered])

  function handleClick(url: string) {
    if (!isDemo) openPr(url)
  }

  function toggleCollapsed(repo: string) {
    setCollapsed(prev => {
      const next = new Set(prev)
      if (next.has(repo)) next.delete(repo)
      else next.add(repo)
      return next
    })
  }

  const totalOpen = prs.filter(pr => pr.state === 'open').length

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
        <Button
          variant={showAll ? 'secondary' : 'ghost'}
          size="sm"
          className="h-8 px-2.5 text-xs"
          onClick={() => setShowAll(v => !v)}
        >
          {showAll ? 'All states' : 'Open only'}
        </Button>
        <span className="text-sm text-muted-foreground">
          {totalOpen} open PR{totalOpen !== 1 ? 's' : ''}
          {showAll && prs.length !== totalOpen && (
            <span className="text-muted-foreground/60"> · {prs.length} total</span>
          )}
        </span>
        {onAddDemo && (
          <Button variant="outline" size="sm" onClick={onAddDemo} className="ml-auto gap-1.5">
            <Plus className="h-4 w-4" />
            Add demo event
          </Button>
        )}
      </div>

      {/* Body */}
      <div className="flex-1 overflow-auto p-4">
        {byRepo.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2">
            <GitPullRequest className="h-8 w-8 opacity-30" />
            <p className="text-sm">
              {filter
                ? 'No pull requests match your filter'
                : showAll
                ? 'No pull requests'
                : 'No open pull requests'}
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {byRepo.map(([repo, repoPrs]) => {
              const isCollapsed = collapsed.has(repo)
              return (
                <section key={repo}>
                  <button
                    className="flex items-center gap-2 mb-2 w-full text-left group"
                    onClick={() => toggleCollapsed(repo)}
                  >
                    <ChevronRight
                      className={`h-3.5 w-3.5 text-muted-foreground/60 transition-transform shrink-0 ${isCollapsed ? '' : 'rotate-90'}`}
                    />
                    <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide group-hover:text-foreground transition-colors">
                      {repo}
                    </h2>
                    <span className="text-xs text-muted-foreground/60">
                      {repoPrs.length} PR{repoPrs.length !== 1 ? 's' : ''}
                    </span>
                  </button>
                  {!isCollapsed && (
                    <div className="grid grid-cols-1 gap-2">
                      {repoPrs.map(pr => (
                        <PrCard key={`${pr.provider}/${pr.repo}/${pr.number}`} pr={pr} onClick={handleClick} />
                      ))}
                    </div>
                  )}
                </section>
              )
            })}
          </div>
        )}
      </div>

      <StatusBar status={status} polling={polling} unread={unread} />
    </div>
  )
}
