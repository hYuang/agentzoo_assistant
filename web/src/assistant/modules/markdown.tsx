import type { AssistantKey } from '../i18n'
import type { ModuleProps } from './types'
import { Notice, useArtifact } from './shared'
import { MathMarkdown } from './mathMarkdown'
import { useAssistantI18n } from '../i18n'
import styles from '../assistant.module.css'

function MarkdownArtifact({
  projectId,
  plugin,
  runId,
  run,
  artifactId,
  emptyKey,
}: ModuleProps & { artifactId: string; emptyKey: AssistantKey }) {
  const { t, artifactLabel } = useAssistantI18n()
  const artifact = run.artifacts?.find((item) => item.id === artifactId) ?? null
  const { content, error } = useArtifact(
    projectId,
    plugin?.id ?? '',
    runId,
    artifactId,
    artifact,
  )
  if (error) return <div className={styles.error}>{error}</div>
  if (!content) {
    return (
      <section className={styles.panel}>
        <h2>{artifactLabel(artifactId)}</h2>
        <Notice>{t(emptyKey)}</Notice>
      </section>
    )
  }
  return (
    <section className={styles.panel}>
      <h2>{artifactLabel(artifactId)}</h2>
      <MathMarkdown text={content} className={styles.md} />
    </section>
  )
}

export function KnowledgeModule(props: ModuleProps) {
  return <MarkdownArtifact {...props} artifactId="knowledge" emptyKey="knowledgeEmpty" />
}

export function ReportModule(props: ModuleProps) {
  return <MarkdownArtifact {...props} artifactId="report" emptyKey="reportEmpty" />
}
