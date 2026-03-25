import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useTheme } from 'next-themes'
import { Plus, Trash2 } from 'lucide-react'
import { toast } from 'sonner'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import type { AppConfig, AppSettings, NotificationMode, RepoConfig } from '@/types'

const EMPTY_REPO: RepoConfig = { provider: 'github', name: '', token: '' }

interface SettingsProps {
  notifMode:         NotificationMode
  onNotifModeChange: (m: NotificationMode) => void
}

export default function Settings({ notifMode, onNotifModeChange }: SettingsProps) {
  const { theme, setTheme } = useTheme()

  const [settings, setSettings] = useState<AppSettings>({
    close_behaviour:   'hide_to_tray',
    notification_mode: notifMode,
  })
  const [port,     setPort]     = useState('7878')
  const [interval, setInterval] = useState('60')
  const [repos,    setRepos]    = useState<RepoConfig[]>([])

  // Load on mount
  useEffect(() => {
    invoke<AppSettings>('get_app_settings').then(setSettings).catch(console.error)
    invoke<AppConfig>('read_config')
      .then(cfg => {
        setPort(String(cfg.daemon_port))
        setInterval(String(cfg.poll_interval_secs))
        setRepos(cfg.repos.map(r => ({ ...r, token: r.token ?? '' })))
      })
      .catch(() => {})
  }, [])

  // ── Helpers ──────────────────────────────────────────────────────────────────

  function updateRepo(i: number, patch: Partial<RepoConfig>) {
    setRepos(prev => prev.map((r, idx) => idx === i ? { ...r, ...patch } : r))
  }

  function removeRepo(i: number) {
    setRepos(prev => prev.filter((_, idx) => idx !== i))
  }

  // ── Auto-save helpers ─────────────────────────────────────────────────────────

  async function saveAppSettings(next: AppSettings) {
    onNotifModeChange(next.notification_mode)
    await invoke('save_app_settings', { settings: next }).catch(console.error)
    toast.success('Settings saved')
  }

  async function saveDaemonConfig(opts?: { port?: string; interval?: string; repos?: RepoConfig[] }) {
    const config: AppConfig = {
      daemon_port:        Number(opts?.port     ?? port)     || 7878,
      poll_interval_secs: Number(opts?.interval ?? interval) || 60,
      theme:              theme ?? 'dark',
      repos:              (opts?.repos ?? repos).map(r => ({ ...r, token: r.token || undefined })),
    }
    await invoke('save_config', { config }).catch(console.error)
    toast.success('Settings saved')
  }

  // Debounce ref for text fields
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  function debouncedSaveConfig(opts: { port?: string; interval?: string; repos?: RepoConfig[] }) {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => saveDaemonConfig(opts), 600)
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-2xl mx-auto py-8 px-6 space-y-6">

        {/* Appearance */}
        <Card>
          <CardHeader><CardTitle>Appearance</CardTitle></CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <Label htmlFor="theme-select">Theme</Label>
              <Select value={theme ?? 'dark'} onValueChange={v => { setTheme(v); saveDaemonConfig() }}>
                <SelectTrigger id="theme-select" className="w-36">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="dark">Dark</SelectItem>
                  <SelectItem value="light">Light</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>

        {/* Notifications */}
        <Card>
          <CardHeader>
            <CardTitle>Notifications</CardTitle>
            <CardDescription>How you want to be alerted about new and updated PRs.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <Label htmlFor="notif-select">Notification style</Label>
              <Select
                value={settings.notification_mode}
                onValueChange={v => {
                  const next = { ...settings, notification_mode: v as NotificationMode }
                  setSettings(next)
                  saveAppSettings(next)
                }}
              >
                <SelectTrigger id="notif-select" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="in_app">In-app toasts</SelectItem>
                  <SelectItem value="os">OS notifications</SelectItem>
                  <SelectItem value="both">Both</SelectItem>
                  <SelectItem value="off">Off</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>

        {/* Behaviour */}
        <Card>
          <CardHeader><CardTitle>Behaviour</CardTitle></CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <div>
                <Label>Hide to tray on close</Label>
                <p className="text-xs text-muted-foreground mt-0.5">
                  When off, closing the window quits the app.
                </p>
              </div>
              <Switch
                checked={settings.close_behaviour === 'hide_to_tray'}
                onCheckedChange={checked => {
                  const next = { ...settings, close_behaviour: checked ? 'hide_to_tray' as const : 'quit' as const }
                  setSettings(next)
                  saveAppSettings(next)
                }}
              />
            </div>
          </CardContent>
        </Card>

        {/* Daemon */}
        <Card>
          <CardHeader>
            <CardTitle>Daemon</CardTitle>
            <CardDescription>Connection and polling settings.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  value={port}
                  onChange={e => { setPort(e.target.value); debouncedSaveConfig({ port: e.target.value }) }}
                  className="font-mono"
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="interval">Poll interval (seconds)</Label>
                <Input
                  id="interval"
                  value={interval}
                  onChange={e => { setInterval(e.target.value); debouncedSaveConfig({ interval: e.target.value }) }}
                  className="font-mono"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Repositories */}
        <Card>
          <CardHeader>
            <CardTitle>Repositories</CardTitle>
            <CardDescription>GitHub / GitLab repos to watch.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {repos.map((repo, i) => (
              <div key={i} className="space-y-2">
                {i > 0 && <Separator />}
                <div className="flex gap-2 items-start pt-2">
                  <Select
                    value={repo.provider}
                    onValueChange={v => {
                      const next = repos.map((r, idx) => idx === i ? { ...r, provider: v } : r)
                      setRepos(next)
                      debouncedSaveConfig({ repos: next })
                    }}
                  >
                    <SelectTrigger className="w-28 shrink-0">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="github">GitHub</SelectItem>
                      <SelectItem value="gitlab">GitLab</SelectItem>
                    </SelectContent>
                  </Select>
                  <Input
                    placeholder="owner/repo"
                    value={repo.name}
                    onChange={e => { updateRepo(i, { name: e.target.value }); debouncedSaveConfig({ repos: repos.map((r, idx) => idx === i ? { ...r, name: e.target.value } : r) }) }}
                    className="font-mono flex-1"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      const next = repos.filter((_, idx) => idx !== i)
                      setRepos(next)
                      saveDaemonConfig({ repos: next })
                    }}
                    className="shrink-0 text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
                <Input
                  placeholder="Personal access token (optional — falls back to env)"
                  type="password"
                  value={repo.token ?? ''}
                  onChange={e => { updateRepo(i, { token: e.target.value }); debouncedSaveConfig({ repos: repos.map((r, idx) => idx === i ? { ...r, token: e.target.value } : r) }) }}
                  className="font-mono text-xs"
                />
              </div>
            ))}

            <Button
              variant="outline"
              size="sm"
              className="w-full mt-2 gap-1.5"
              onClick={() => {
                const next = [...repos, { ...EMPTY_REPO }]
                setRepos(next)
                saveDaemonConfig({ repos: next })
              }}
            >
              <Plus className="h-4 w-4" />
              Add repository
            </Button>
          </CardContent>
        </Card>

      </div>
    </div>
  )
}
