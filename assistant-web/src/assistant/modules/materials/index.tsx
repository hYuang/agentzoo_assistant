import type { ModuleProps } from '../types'
import { Notice, RawView, safeJson, useArtifact } from '../shared'
import { MathMarkdown } from '../mathMarkdown'
import { useAssistantI18n } from '../../i18n'
import styles from '../../assistant.module.css'

interface MaterialItem {
  id?: string
  title?: string
  kind?: string
  location?: string
  summary?: string
  key_points?: string[]
}

function items(content: string): MaterialItem[] | null {
  const parsed = safeJson(content)
  if (!parsed) return null
  if (Array.isArray(parsed.items)) return parsed.items as MaterialItem[]
  if (Array.isArray(parsed)) return parsed as MaterialItem[]
  return null
}

export function MaterialsModule({ projectId, runId, run }: ModuleProps) {
  const { t, artifactLabel } = useAssistantI18n()
  const artifact = run.artifacts?.find((item) => item.id === 'materials') ?? null
  const { content, error } = useArtifact(projectId, runId, 'materials', artifact)
  if (error) return <div className={styles.error}>{error}</div>
  if (!content) {
    return (
      <section className={styles.panel}>
        <h2>{artifactLabel('materials')}</h2>
        <Notice>{t('materialsEmpty')}</Notice>
      </section>
    )
  }
  const list = items(content)
  if (!list) return <RawView title={artifactLabel('materials')} content={content} />
  return (
    <section className={styles.panel}>
      <h2>{artifactLabel('materials')}</h2>
      <div className={styles.itemGrid}>
        {list.map((item, index) => (
          <article key={item.id ?? index} className={styles.itemCard}>
            <div className={styles.itemHead}>
              <strong>
                <MathMarkdown
                  text={item.title || item.location || `#${index + 1}`}
                  inline
                />
              </strong>
              {item.kind && <span className={styles.tag}>{item.kind}</span>}
            </div>
            {item.location && (
              <div className={styles.mono}>
                <MathMarkdown text={item.location} inline />
              </div>
            )}
            {item.summary && (
              <div className={styles.itemSummary}>
                <MathMarkdown text={item.summary} inline />
              </div>
            )}
            {Array.isArray(item.key_points) && item.key_points.length > 0 && (
              <ul className={styles.itemList}>
                {item.key_points.map((point, i) => (
                  <li key={i}>
                    <MathMarkdown text={point} inline />
                  </li>
                ))}
              </ul>
            )}
          </article>
        ))}
      </div>
    </section>
  )
}
