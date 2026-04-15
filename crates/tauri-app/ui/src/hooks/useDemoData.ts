import { useState, useCallback } from 'react'
import type { GithubNotification, PullRequest } from '../types'

const AUTHORS  = ['alice', 'bob', 'carol', 'dave', 'eve', 'frank', 'grace']
const REPOS    = ['acme/frontend', 'acme/api', 'acme/infra', 'acme/mobile', 'acme/docs']
const STATES   = ['open', 'open', 'open', 'merged', 'closed'] as const
const TITLES   = [
  'Add dark mode support',
  'Fix login race condition',
  'Refactor auth middleware',
  'Update dependencies',
  'Improve error messages',
  'Add pagination to list endpoint',
  'Fix memory leak in worker pool',
  'Add unit tests for core logic',
  'Bump CI to Node 20',
  'Remove deprecated API calls',
  'Implement rate limiting',
  'Add WebSocket reconnection',
]

const NOTIF_ENTRIES: Array<{ reason: string; subject_type: string; subject_title: string }> = [
  { reason: 'comment',          subject_type: 'PullRequest', subject_title: 'Fix login race condition' },
  { reason: 'mention',          subject_type: 'Issue',       subject_title: 'Performance degradation under high load' },
  { reason: 'review_requested', subject_type: 'PullRequest', subject_title: 'Refactor auth middleware' },
  { reason: 'ci_activity',      subject_type: 'CheckSuite',  subject_title: 'build / test (push)' },
  { reason: 'assign',           subject_type: 'PullRequest', subject_title: 'Implement rate limiting' },
  { reason: 'comment',          subject_type: 'Issue',       subject_title: 'Webhook delivery failures since deploy' },
  { reason: 'mention',          subject_type: 'PullRequest', subject_title: 'Add WebSocket reconnection' },
  { reason: 'ci_activity',      subject_type: 'CheckSuite',  subject_title: 'deploy / staging (push)' },
]

let seq = 1

function fakePr(): PullRequest {
  const id = seq++
  return {
    id,
    number: id,
    title: TITLES[(id - 1) % TITLES.length],
    state: STATES[(id - 1) % STATES.length],
    url: 'https://github.com',
    author: AUTHORS[(id - 1) % AUTHORS.length],
    repo: REPOS[(id - 1) % REPOS.length],
    provider: 'github',
    created_at: Math.floor(Date.now() / 1000) - ((id - 1) * 3600 * 7),
    draft: id % 7 === 0,
    reviewers: [],
    assignees: [],
  }
}

function fakeNotification(idSuffix: string | number): GithubNotification {
  const id = seq++
  const entry = NOTIF_ENTRIES[(id - 1) % NOTIF_ENTRIES.length]
  return {
    id: `demo-notif-${idSuffix}`,
    repo: REPOS[(id - 1) % REPOS.length],
    subject_type:  entry.subject_type,
    subject_title: entry.subject_title,
    reason:        entry.reason,
    url: 'https://github.com',
    updated_at: Math.floor(Date.now() / 1000),
    seen:   false,
    hidden: false,
  }
}

function seedEvents(): GithubNotification[] {
  const now = Math.floor(Date.now() / 1000)
  return [
    { id: 'seed-1', repo: 'acme/api',      subject_type: 'PullRequest', subject_title: 'Fix login race condition',          reason: 'comment',          url: '', updated_at: now - 120,  seen: false, hidden: false },
    { id: 'seed-2', repo: 'acme/infra',    subject_type: 'Issue',       subject_title: 'Performance degradation under load', reason: 'mention',          url: '', updated_at: now - 310,  seen: false, hidden: false },
    { id: 'seed-3', repo: 'acme/frontend', subject_type: 'PullRequest', subject_title: 'Refactor auth middleware',           reason: 'review_requested', url: '', updated_at: now - 600,  seen: false, hidden: false },
    { id: 'seed-4', repo: 'acme/mobile',   subject_type: 'CheckSuite',  subject_title: 'build / test (push)',                reason: 'ci_activity',      url: '', updated_at: now - 950,  seen: false, hidden: false },
    { id: 'seed-5', repo: 'acme/api',      subject_type: 'PullRequest', subject_title: 'Implement rate limiting',            reason: 'assign',           url: '', updated_at: now - 1800, seen: false, hidden: false },
    { id: 'seed-6', repo: 'acme/docs',     subject_type: 'Issue',       subject_title: 'Webhook delivery failures',          reason: 'comment',          url: '', updated_at: now - 3600, seen: false, hidden: false },
  ]
}

export type DemoItem =
  | { kind: 'pr';           pr: PullRequest }
  | { kind: 'notification'; notification: GithubNotification }

export function useDemoData() {
  const [prs,    setPrs]    = useState<PullRequest[]>(() => Array.from({ length: 6 }, () => fakePr()))
  const [events, setEvents] = useState<GithubNotification[]>(seedEvents)

  const addItem = useCallback((): DemoItem => {
    if (seq % 3 === 0) {
      const notification = fakeNotification(seq)
      setEvents(prev => [notification, ...prev].slice(0, 50))
      return { kind: 'notification', notification }
    }
    const pr = fakePr()
    setPrs(prev => [pr, ...prev])
    return { kind: 'pr', pr }
  }, [])

  return { prs, events, addItem }
}
