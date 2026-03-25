import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatAge(createdAt: number): string {
  const secs = Math.floor(Date.now() / 1000) - createdAt
  if (secs < 60)         return `${secs}s`
  if (secs < 3600)       return `${Math.floor(secs / 60)}m`
  if (secs < 86400)      return `${Math.floor(secs / 3600)}h`
  if (secs < 86400 * 30) return `${Math.floor(secs / 86400)}d`
  return `${Math.floor(secs / (86400 * 7))}w`
}

export function formatDate(createdAt: number): string {
  return new Date(createdAt * 1000).toLocaleString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit',
  })
}
