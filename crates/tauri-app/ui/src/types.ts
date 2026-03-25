export interface PullRequest {
  id: number
  number: number
  title: string
  state: string
  url: string
  author: string
  repo: string
  provider: string
  created_at: number
  draft: boolean
  reviewers: string[]
  assignees: string[]
}

export interface RepoConfig {
  provider: string
  name: string
  token?: string
}

export interface AppConfig {
  daemon_port: number
  poll_interval_secs: number
  repos: RepoConfig[]
  theme: string
}

export type NotificationMode = 'in_app' | 'os' | 'both' | 'off'

export interface AppSettings {
  close_behaviour:   'hide_to_tray' | 'quit'
  notification_mode: NotificationMode
}

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected'
