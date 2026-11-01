import { useEffect, useState } from 'react'
import type { ModuleProps } from '../modules/types'
import { useAssistantI18n } from '../i18n'
import { useStudioSession } from './session'
import {
  publishToSynapse,
  type SynapsePublishMode,
  type SynapseRecord,
} from './api'
import styles from '../assistant.module.css'

export function SynapseModule({ projectId, plugin, runId, run }: ModuleProps) {
  const { t } = useAssistantI18n()
  const { status, session, canPublish, refresh } = useStudioSession()
  const [record, setRecord] = useState<SynapseRecord | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const pluginId = plugin?.id ?? ''
  const quizReady = Boolean(
    run.artifacts?.some(
      (artifact) => artifact.id === 'quiz' && artifact.status !== 'empty',
    ),
  )

  useEffect(() => {
    setRecord(status?.record ?? null)
  }, [status?.record])

  const publish = async (mode: SynapsePublishMode) => {
    if (!pluginId) return
    setBusy(true)
    setError(null)
    try {
      const result = await publishToSynapse(projectId, pluginId, runId, mode)
      setRecord(result.record)
      await refresh()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const configured = status?.config.configured ?? false
  const needsLogin = !session && !configured

  return (
    <section className={styles.panel}>
      <div className={styles.quizHead}>
        <div>
          <h2>{t('synapseTitle')}</h2>
          <p className={styles.muted}>{t('synapseHint')}</p>
        </div>
        <div className={styles.quizActions}>
          {record && (
            <button
              type="button"
              className={styles.btn}
              disabled={busy || !quizReady || !canPublish || !pluginId}
              onClick={() => void publish('overwrite')}
            >
              {busy ? t('synapseSyncing') : t('synapseOverwrite')}
            </button>
          )}
          <button
            type="button"
            className={styles.primary}
            disabled={busy || !quizReady || !canPublish || !pluginId}
            onClick={() => void publish('add')}
          >
            {busy
              ? t('synapseSyncing')
              : record
                ? t('synapseAdd')
                : t('synapseSync')}
          </button>
        </div>
      </div>

      {configured && status && (
        <p className={styles.muted}>
          {t('synapseConfigured', {
            url: status.config.gateway_url,
            tenant: status.config.tenant_id,
          })}
        </p>
      )}
      {needsLogin && <div className={styles.notice}>{t('synapseNeedLogin')}</div>}
      {!quizReady && <div className={styles.notice}>{t('synapseNoQuiz')}</div>}
      {error && <div className={styles.error}>{error}</div>}

      {record && (
        <>
          <div className={styles.metrics}>
            <div className={styles.metric}>
              <div className={styles.metricLabel}>{t('synapsePaper')}</div>
              <div className={styles.metricValue}>#{record.paper_id}</div>
            </div>
            <div className={styles.metric}>
              <div className={styles.metricLabel}>{t('synapseQuestions')}</div>
              <div className={styles.metricValue}>{record.question_count}</div>
            </div>
            <div className={styles.metric}>
              <div className={styles.metricLabel}>{t('synapseKnowledge')}</div>
              <div className={styles.metricValue}>{record.knowledge_point_count}</div>
            </div>
            <div className={styles.metric}>
              <div className={styles.metricLabel}>{t('synapseLastSync')}</div>
              <div className={styles.metricValue}>
                {new Date(record.synced_at).toLocaleString()}
              </div>
            </div>
          </div>

        </>
      )}

      {record && <p className={styles.muted}>{t('synapseAlready')}</p>}
    </section>
  )
}
