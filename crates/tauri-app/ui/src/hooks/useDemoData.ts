import { useState, useCallback } from 'react'
import type { PullRequest } from '../types'

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
    // spread creation times across the last 30 days
    created_at: Math.floor(Date.now() / 1000) - ((id - 1) * 3600 * 7),
    draft: id % 7 === 0,
    reviewers: [],
    assignees: [],
  }
}

export function useDemoData() {
  const [prs, setPrs] = useState<PullRequest[]>(() =>
    Array.from({ length: 6 }, () => fakePr()),
  )

  const addPr = useCallback((): PullRequest => {
    const pr = fakePr()
    setPrs(prev => [pr, ...prev])
    return pr
  }, [])

  return { prs, addPr }
}
