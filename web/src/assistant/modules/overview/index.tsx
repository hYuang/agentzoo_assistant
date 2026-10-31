import { useNavigate } from 'react-router-dom'
import type { ModuleProps } from '../types'
import { Badge, formatSize, stageState } from '../shared'
import { useAssistantI18n } from '../../i18n'
import { useChat } from '@agentzoo/web-ui'
import styles from '../../assistant.module.css'

export function OverviewModule({
  projectId,
  runId,
  workflow,
  run,
  onAct,
  onOpenArtifact,
}: ModuleProps) {
  const { t, stageLabel, artifactLabel, statusLabel } = useAssistantI18n()
  const navigate = useNavigate()
  const { sessions } = useChat()
  const completed = new Set(run.completed_stages ?? [])
  const total = workflow?.stages?.length ?? 0
  const done = completed.size
  const pct = total ? Math.round((done / total) * 100) : 0
  const current = workflow?.stages?.find((stage) => stage.id === run.stage)
  const next = (workflow?.stages ?? []).find((stage) => stageState(stage, run) !== 'done')

  return (
    <>
      <div className={styles.metrics}>
        <Metric label={t('status')} value={statusLabel(run.status)} />
        <Metric label={t('flowStages')} value={`${done}/${total}`} />
        <Metric
          label={t('currentStage')}
          value={current ? stageLabel(current.id) : run.status === 'completed' ? t('completed') : t('statusCreated')}
        />
        <Metric
          label={t('artifacts')}
          value={String((run.artifacts ?? []).filter((a) => a.status !== 'empty').length)}
        />
      </div>

      <div className={styles.grid2}>
        <section className={styles.panel}>
          <h2>{t('nextStep')}</h2>
          <p className={styles.muted}>{t('startHint')}</p>
          {run.status === 'waiting_gate' && current && (
            <p className={styles.muted}>
              {stageLabel(current.id)} · {t('waiting')}
            </p>
          )}
          {run.status === 'completed' && <p className={styles.muted}>{t('statusCompleted')}</p>}
          {['created', 'failed', 'cancelled'].includes(run.status) && (
            <button
              type="button"
              className={styles.primary}
              onClick={() => onAct('start')}
            >
              {t('start')}
            </button>
          )}
          {run.session_id && (
            <button
              className={styles.btn}
              onClick={() => {
                if (!projectId || !run.session_id) return
                const s = sessions.find((x) => x.session_id === run.session_id)
                const tid = s?.conversation_id ?? run.session_id
                navigate(
                  `/project/${encodeURIComponent(projectId)}/topic/${encodeURIComponent(tid)}`,
                )
              }}
            >
              {t('openChat')}
            </button>
          )}
        </section>

        <section className={styles.panel}>
          <h2>{t('overallProgress')}</h2>
          <div className={styles.progress}>
            <span className={styles.progressFill} style={{ width: `${pct}%` }} />
          </div>
          <p className={styles.muted}>{pct}%</p>
          <p className={styles.meta}>
            <Badge status={run.status} />
            <span>
              {run.completed_stages?.length ?? 0} {t('stagesCompleted')}
            </span>
          </p>
        </section>
      </div>

      <section className={styles.panel}>
        <h2>{t('artifacts')}</h2>
        <div className={styles.artifactGrid}>
          {(run.artifacts ?? []).map((artifact) => (
            <button
              key={artifact.id}
              type="button"
              className={styles.artifact}
              onClick={() => onOpenArtifact(artifact.id)}
            >
              <span className={styles.artifactName}>{artifactLabel(artifact.id)}</span>
              <span className={styles.artifactState}>
                {artifact.status} · {formatSize(artifact.size)}
              </span>
            </button>
          ))}
        </div>
      </section>
    </>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.metric}>
      <div className={styles.metricLabel}>{label}</div>
      <div className={styles.metricValue}>{value}</div>
    </div>
  )
}
