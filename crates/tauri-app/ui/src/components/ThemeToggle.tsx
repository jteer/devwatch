import { Moon, Sun, Monitor } from 'lucide-react'
import { useTheme } from 'next-themes'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

const THEMES = ['light', 'dark', 'system'] as const
type Theme = typeof THEMES[number]

const icons: Record<Theme, React.ReactNode> = {
  light:  <Sun className="h-4 w-4" />,
  dark:   <Moon className="h-4 w-4" />,
  system: <Monitor className="h-4 w-4" />,
}

const labels: Record<Theme, string> = {
  light:  'Light',
  dark:   'Dark',
  system: 'System',
}

export function ThemeToggle() {
  const { theme, setTheme } = useTheme()
  const current = (theme ?? 'dark') as Theme
  const next = THEMES[(THEMES.indexOf(current) + 1) % THEMES.length]

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" onClick={() => setTheme(next)} aria-label="Toggle theme">
          {icons[current]}
        </Button>
      </TooltipTrigger>
      <TooltipContent>Theme: {labels[current]} (click for {labels[next]})</TooltipContent>
    </Tooltip>
  )
}
