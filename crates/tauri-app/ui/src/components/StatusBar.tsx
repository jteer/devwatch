import { cn } from '@/lib/utils'
import type { ConnectionStatus } from '@/types'

interface StatusBarProps {
  status: ConnectionStatus
  polling: boolean
  unread: number
}

const statusConfig: Record<ConnectionStatus, { label: string; dot: string }> = {
  connected:    { label: 'Connected',    dot: 'bg-green-500' },
  connecting:   { label: 'Connecting…',  dot: 'bg-yellow-500 animate-pulse' },
  disconnected: { label: 'Disconnected', dot: 'bg-red-500' },
}

export function StatusBar({ status, polling, unread }: StatusBarProps) {
  const { label, dot } = statusConfig[status]

  return (
    <footer className="flex items-center justify-between px-4 py-1.5 border-t text-xs text-muted-foreground shrink-0">
      <div className="flex items-center gap-2">
        <span className={cn('inline-block h-2 w-2 rounded-full', dot)} />
        <span>{polling ? 'Polling…' : label}</span>
      </div>
      {unread > 0 && (
        <span>{unread} unread event{unread !== 1 ? 's' : ''}</span>
      )}
    </footer>
  )
}
