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
import { ArrowUp, ArrowDown, ArrowUpDown } from 'lucide-react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { formatAge, formatDate } from '@/lib/utils'
import type { PullRequest } from '@/types'

const col = createColumnHelper<PullRequest>()

function stateVariant(state: string) {
  if (state === 'open')   return 'success'
  if (state === 'merged') return 'merged'
  return 'destructive'
}

interface PrTableProps {
  prs: PullRequest[]
  filter: string
  onRowClick: (pr: PullRequest) => void
}

export function PrTable({ prs, filter, onRowClick }: PrTableProps) {
  const [sorting, setSorting] = useState<SortingState>([])

  const columns = useMemo(() => [
    col.accessor('number', {
      header: '#',
      cell: i => <span className="text-muted-foreground font-mono">#{i.getValue()}</span>,
      size: 64,
    }),
    col.accessor('repo', {
      header: 'Repo',
      size: 200,
      cell: i => <span className="text-muted-foreground text-xs">{i.getValue()}</span>,
    }),
    col.accessor('title', {
      header: 'Title',
      cell: i => {
        const pr = i.row.original
        return (
          <span className="flex items-center gap-2">
            {pr.draft && <Badge variant="outline" className="text-[10px] py-0 px-1.5 shrink-0">draft</Badge>}
            <span className="truncate">{i.getValue()}</span>
          </span>
        )
      },
    }),
    col.accessor('author', {
      header: 'Author',
      size: 120,
    }),
    col.accessor('created_at', {
      id: 'age',
      header: 'Age',
      size: 64,
      cell: i => (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="text-muted-foreground cursor-default">{formatAge(i.getValue())}</span>
          </TooltipTrigger>
          <TooltipContent>{formatDate(i.getValue())}</TooltipContent>
        </Tooltip>
      ),
      sortingFn: (a, b) => a.original.created_at - b.original.created_at,
    }),
    col.accessor('state', {
      header: 'State',
      size: 80,
      cell: i => <Badge variant={stateVariant(i.getValue())}>{i.getValue()}</Badge>,
    }),
  ], [])

  const table = useReactTable({
    data: prs,
    columns,
    state: { sorting, globalFilter: filter },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    globalFilterFn: (row, _colId, filterValue: string) => {
      const q = filterValue.toLowerCase()
      const { title, author, repo } = row.original
      return title.toLowerCase().includes(q)
        || author.toLowerCase().includes(q)
        || repo.toLowerCase().includes(q)
    },
  })

  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map(hg => (
          <TableRow key={hg.id}>
            {hg.headers.map(header => (
              <TableHead key={header.id} style={{ width: header.getSize() }}>
                {header.isPlaceholder ? null : (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="-ml-3 h-8 font-medium text-muted-foreground hover:text-foreground"
                    onClick={header.column.getToggleSortingHandler()}
                  >
                    {flexRender(header.column.columnDef.header, header.getContext())}
                    {header.column.getCanSort() && (
                      header.column.getIsSorted() === 'asc'  ? <ArrowUp   className="ml-1.5 h-3.5 w-3.5" /> :
                      header.column.getIsSorted() === 'desc' ? <ArrowDown className="ml-1.5 h-3.5 w-3.5" /> :
                      <ArrowUpDown className="ml-1.5 h-3.5 w-3.5 opacity-30" />
                    )}
                  </Button>
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
              No pull requests
            </TableCell>
          </TableRow>
        ) : (
          table.getRowModel().rows.map(row => (
            <TableRow
              key={row.id}
              className="cursor-pointer"
              onClick={() => onRowClick(row.original)}
            >
              {row.getVisibleCells().map(cell => (
                <TableCell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))
        )}
      </TableBody>
    </Table>
  )
}
