import { useEffect, useState, type ReactNode } from 'react'
import { ApiError, readPluginArtifact } from '@agentzoo/web-ui'
import type {
  PluginArtifactInfo,
  PluginRunInfo,
  PluginStageInfo,
  PluginWorkflowInfo,
} from '@agentzoo/web-ui'
import { useAssistantI18n } from '../i18n'
import styles from '../assistant.module.css'

export function stageState(stage: PluginStageInfo, run: PluginRunInfo): string {
  if (run.stage === stage.id) {
    return run.status === 'waiting_gate' ? 'waiting' : 'current'
  }
  const completed = new Set(run.completed_stages ?? [])
  if (completed.has(stage.id)) return 'done'
  return 'pending'
}

export function safeJson(content: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(content)
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, unknown>) : null
  } catch {
    return null
  }
}

export function formatSize(size?: number): string {
  if (!size) return '0 B'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

export function Badge({ status }: { status: string }) {
  const { statusLabel } = useAssistantI18n()
  const cls =
    status === 'completed'
      ? `${styles.badge} ${styles.badgeDone}`
      : status === 'failed'
        ? `${styles.badge} ${styles.badgeFail}`
        : status === 'waiting_gate'
          ? `${styles.badge} ${styles.badgeGate}`
          : styles.badge
  return <span className={cls}>{statusLabel(status)}</span>
}

export function StatusPill({ state }: { state: string }) {
  const { t } = useAssistantI18n()
  const label =
    {
      done: t('completed'),
      current: t('running'),
      waiting: t('waiting'),
      pending: t('pending'),
    }[state] ?? state
  const cls =
    state === 'done'
      ? `${styles.pill} ${styles.pillDone}`
      : state === 'waiting'
        ? `${styles.pill} ${styles.pillGate}`
        : styles.pill
  return <span className={cls}>{label}</span>
}

export function Notice({ children }: { children: ReactNode }) {
  return <div className={styles.notice}>{children}</div>
}

export function RawView({ title, content }: { title: string; content: string }) {
  return (
    <section className={styles.panel}>
      <h2>{title}</h2>
      <pre className={styles.pre}>{content}</pre>
    </section>
  )
}

export function useArtifact(
  projectId: string,
  runId: string,
  id: string,
  artifact: PluginArtifactInfo | null,
) {
  const [content, setContent] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    setContent(null)
    setError(null)
    if (!artifact || artifact.path.endsWith('/') || artifact.path.endsWith('\\')) {
      return
    }
    readPluginArtifact(projectId, 'assistant', runId, id)
      .then((res) => {
        if (!cancelled) setContent(res.encoding === 'utf-8' ? res.content : null)
      })
      .catch((e: unknown) => {
        if (cancelled) return
        // A stage has not produced this artifact yet: show the module's empty
        // state instead of a raw "file not found" error.
        if (
          e instanceof ApiError &&
          (e.code === 'file_not_found' || e.code === 'plugin_artifact_not_found')
        ) {
          setContent(null)
        } else {
          setError(String(e))
        }
      })
    return () => {
      cancelled = true
    }
  }, [projectId, runId, id, artifact])
  return { content, error }
}
