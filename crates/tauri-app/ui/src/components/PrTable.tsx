import { useMemo, useState } from 'react'
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table'
import {
  ArrowUp, ArrowDown, ArrowUpDown,
  Github, Gitlab,
  GitPullRequest, MessageSquare, AtSign, Eye, GitBranch, UserCheck, Bell,
  Check, X, Undo2,
} from 'lucide-react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { formatAge, formatDate } from '@/lib/utils'
import { reasonLabel } from '@/hooks/useNotifications'
import type { GithubNotification, PullRequest } from '@/types'

// ── Unified feed item ─────────────────────────────────────────────────────────

export type FeedItem =
  | { kind: 'pr';           data: PullRequest }
  | { kind: 'notification'; data: GithubNotification }

function itemRepo(item: FeedItem)  { return item.data.repo }
function itemTitle(item: FeedItem) { return item.kind === 'pr' ? item.data.title        : item.data.subject_title }
function itemFrom(item: FeedItem)  { return item.kind === 'pr' ? item.data.author       : reasonLabel(item.data.reason) }
function itemAge(item: FeedItem)   { return item.kind === 'pr' ? item.data.created_at   : item.data.updated_at }

// ── Icons / badges ────────────────────────────────────────────────────────────

function KindIcon({ item }: { item: FeedItem }) {
  const cls = 'h-3.5 w-3.5'
  if (item.kind === 'pr') return <GitPullRequest className={`${cls} text-muted-foreground`} />
  switch ((item.data as GithubNotification).reason) {
    case 'comment':          return <MessageSquare className={`${cls} text-blue-500`} />
    case 'mention':          return <AtSign        className={`${cls} text-amber-500`} />
    case 'review_requested': return <Eye           className={`${cls} text-violet-500`} />
    case 'ci_activity':      return <GitBranch     className={`${cls} text-emerald-500`} />
    case 'assign':           return <UserCheck     className={`${cls} text-sky-500`} />
    default:                 return <Bell          className={`${cls} text-muted-foreground`} />
  }
}

function StatusBadge({ item }: { item: FeedItem }) {
  if (item.kind === 'pr') {
    const state = (item.data as PullRequest).state
    const variant = state === 'open' ? 'success' : state === 'merged' ? 'merged' : 'destructive'
    return <Badge variant={variant}>{state}</Badge>
  }
  const reason = (item.data as GithubNotification).reason
  const variant =
    reason === 'comment'          ? 'secondary' :
    reason === 'mention'          ? 'warning'   :
    reason === 'review_requested' ? 'merged'    :
    reason === 'ci_activity'      ? 'success'   :
    'outline'
  const label =
    reason === 'comment'          ? 'comment'  :
    reason === 'mention'          ? 'mention'  :
    reason === 'review_requested' ? 'review'   :
    reason === 'ci_activity'      ? 'CI'       :
    reason === 'assign'           ? 'assigned' :
    reason
  return <Badge variant={variant}>{label}</Badge>
}

function SubjectTypeBadge({ type }: { type: string }) {
  const cls = 'text-[10px] py-0 px-1.5 shrink-0'
  switch (type) {
    case 'PullRequest':
      return <Badge variant="merged"    className={cls}>PR</Badge>
    case 'Issue':
      return <Badge variant="secondary" className={`${cls} text-blue-500 border-blue-500/30 bg-blue-500/10`}>issue</Badge>
    case 'CheckSuite':
    case 'WorkflowRun':
      return <Badge variant="success"   className={cls}>CI</Badge>
    case 'Release':
      return <Badge variant="warning"   className={cls}>release</Badge>
    case 'Commit':
      return <Badge variant="outline"   className={`${cls} text-muted-foreground`}>commit</Badge>
    case 'Discussion':
      return <Badge variant="outline"   className={`${cls} text-violet-500 border-violet-500/30 bg-violet-500/10`}>discussion</Badge>
    default:
      return <Badge variant="outline"   className={`${cls} text-muted-foreground`}>{type.toLowerCase()}</Badge>
  }
}

// ── Table ─────────────────────────────────────────────────────────────────────

const col = createColumnHelper<FeedItem>()

interface FeedTableProps {
  items: FeedItem[]
  filter: string
  onRowClick: (item: FeedItem) => void
  seenIds?: Set<string>
  onToggleSeen?: (id: string) => void
  onHide?: (id: string) => void
  onUnhide?: (id: string) => void
}

export function FeedTable({ items, filter, onRowClick, seenIds, onToggleSeen, onHide, onUnhide }: FeedTableProps) {
  const [sorting, setSorting] = useState<SortingState>([])

  const columns = useMemo(() => [
    col.accessor(itemRepo, {
      id: 'repo',
      header: 'Repo',
      size: 75,
      cell: i => {
        const item = i.row.original
        const provider = item.kind === 'pr' ? item.data.provider : 'github'
        const ProviderIcon = provider === 'gitlab' ? Gitlab : Github
        return (
          <span className="flex items-center gap-1.5 text-xs min-w-0">
            <KindIcon item={item} />
            <ProviderIcon className="h-3 w-3 shrink-0 text-muted-foreground opacity-70" />
            <span className="text-muted-foreground truncate">{itemRepo(item)}</span>
          </span>
        )
      },
    }),
    col.accessor(itemTitle, {
      id: 'title',
      header: 'Title',
      size: 400,
      cell: i => {
        const item = i.row.original
        const title = itemTitle(item)
        const isDraft = item.kind === 'pr' && (item.data as PullRequest).draft
        const subjectType = item.kind === 'notification'
          ? (item.data as GithubNotification).subject_type
          : null
        const isSeen = item.kind === 'notification' && (seenIds?.has(item.data.id) ?? false)
        return (
          <span className="flex items-center gap-2 min-w-0">
            {isDraft && (
              <Badge variant="outline" className="text-[10px] py-0 px-1.5 shrink-0">draft</Badge>
            )}
            {subjectType && (
              <SubjectTypeBadge type={subjectType} />
            )}
            {isSeen && (
              <Check className="h-3 w-3 shrink-0 text-muted-foreground/60" />
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="truncate">{title}</span>
              </TooltipTrigger>
              <TooltipContent className="max-w-sm break-words">{title}</TooltipContent>
            </Tooltip>
          </span>
        )
      },
    }),
    col.accessor(itemFrom, {
      id: 'from',
      header: 'From',
      size: 140,
      cell: i => <span className="text-sm">{itemFrom(i.row.original)}</span>,
    }),
    col.accessor(itemAge, {
      id: 'age',
      header: 'Age',
      size: 64,
      cell: i => (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="text-muted-foreground cursor-default">{formatAge(itemAge(i.row.original))}</span>
          </TooltipTrigger>
          <TooltipContent>{formatDate(itemAge(i.row.original))}</TooltipContent>
        </Tooltip>
      ),
    }),
    col.display({
      id: 'status',
      header: 'Status',
      size: 90,
      enableSorting: false,
      cell: i => <StatusBadge item={i.row.original} />,
    }),
    col.display({
      id: 'actions',
      header: 'Actions',
      size: 56,
      enableSorting: false,
      cell: i => {
        const item = i.row.original
        if (item.kind !== 'notification') return null
        const id = item.data.id
        const seen   = seenIds?.has(id) ?? false
        const hidden = (item.data as GithubNotification).hidden
        return (
          <span className="flex items-center gap-0.5">
            {hidden ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 text-muted-foreground hover:text-foreground"
                    onClick={e => { e.stopPropagation(); onUnhide?.(id) }}
                  >
                    <Undo2 className="h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Restore</TooltipContent>
              </Tooltip>
            ) : (
              <>
                {!seen && (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-muted-foreground"
                        onClick={e => { e.stopPropagation(); onToggleSeen?.(id) }}
                      >
                        <Check className="h-3.5 w-3.5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>Mark seen</TooltipContent>
                  </Tooltip>
                )}
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-destructive"
                      onClick={e => { e.stopPropagation(); onHide?.(id) }}
                    >
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Hide</TooltipContent>
                </Tooltip>
              </>
            )}
          </span>
        )
      },
    }),
  ], [seenIds, onToggleSeen, onHide, onUnhide])

  const table = useReactTable({
    data: items,
    columns,
    state: { sorting, globalFilter: filter },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    globalFilterFn: (row, _colId, filterValue: string) => {
      const q = filterValue.toLowerCase()
      const item = row.original
      return itemRepo(item).toLowerCase().includes(q)
        || itemTitle(item).toLowerCase().includes(q)
        || itemFrom(item).toLowerCase().includes(q)
    },
  })

  return (
    <Table className="table-fixed">
      <TableHeader className="sticky top-0 z-10 bg-background">
        {table.getHeaderGroups().map(hg => (
          <TableRow key={hg.id}>
            {hg.headers.map(header => (
              <TableHead key={header.id} style={{ width: header.getSize() }}>
                {header.isPlaceholder ? null : (
                  header.column.getCanSort() ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="-ml-3 h-8 font-medium text-muted-foreground hover:text-foreground"
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      {flexRender(header.column.columnDef.header, header.getContext())}
                      {header.column.getIsSorted() === 'asc'  ? <ArrowUp   className="ml-1.5 h-3.5 w-3.5" /> :
                       header.column.getIsSorted() === 'desc' ? <ArrowDown className="ml-1.5 h-3.5 w-3.5" /> :
                       <ArrowUpDown className="ml-1.5 h-3.5 w-3.5 opacity-30" />}
                    </Button>
                  ) : (
                    <span className="text-xs font-medium text-muted-foreground px-3">
                      {flexRender(header.column.columnDef.header, header.getContext())}
                    </span>
                  )
                )}
              </TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.length === 0 ? (
          <TableRow>
            <TableCell colSpan={columns.length} className="h-24 text-center text-muted-foreground">
              No pull requests or notifications
            </TableCell>
          </TableRow>
        ) : (
          table.getRowModel().rows.map(row => {
            const item = row.original
            const isSeen   = item.kind === 'notification' && (seenIds?.has(item.data.id) ?? false)
            const isHidden = item.kind === 'notification' && (item.data as GithubNotification).hidden
            return (
              <TableRow
                key={row.id}
                className={`cursor-pointer group/row${isHidden ? ' opacity-40 bg-muted/20 italic' : isSeen ? ' opacity-50 bg-muted/30' : ''}`}
                onClick={() => onRowClick(row.original)}
              >
                {row.getVisibleCells().map(cell => (
                  <TableCell
                    key={cell.id}
                    className={cell.column.id === 'title' || cell.column.id === 'repo' ? 'max-w-0' : undefined}
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            )
          })
        )}
      </TableBody>
    </Table>
  )
}
