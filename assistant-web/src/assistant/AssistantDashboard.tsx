import { useCallback, useEffect, useState } from 'react'
import {
  BookOpen,
  ChevronDown,
  FileText,
  FolderOpen,
  LayoutDashboard,
  Library,
  ListChecks,
  Network,
  RotateCcw,
  type LucideIcon,
} from 'lucide-react'
import {
  approvePluginRun,
  listPluginRuns,
  rejectPluginRun,
  startPluginRun,
} from '@agentzoo/web-ui'
import { useChat } from '@agentzoo/web-ui'
import type { PluginDashboardProps } from '../contract'
import type { PluginRunInfo } from '@agentzoo/web-ui'
import { Badge, stageState } from './modules/shared'
import { moduleById, moduleForStage, workbenchModules } from './modules/registry'
import { useAssistantI18n } from './i18n'
import styles from './assistant.module.css'

const STAGE_ICONS: Record<string, LucideIcon> = {
  intake: BookOpen,
  map: Network,
  practice: ListChecks,
  review: RotateCcw,
  report: FileText,
}

const WORKBENCH_ICONS: Record<string, LucideIcon> = {
  overview: LayoutDashboard,
  knowledge_base: Library,
  materials: FolderOpen,
  knowledge: Network,
  practice: ListChecks,
  review: RotateCcw,
  report: FileText,
}

const LEARNING_WORKFLOW = 'timed_practice_flow'

function isLearningRun(run: PluginRunInfo): boolean {
  return run.workflow_id === LEARNING_WORKFLOW || run.entry_id === 'learning'
}

const MODULE_KEYS: Record<
  string,
  | 'moduleOverview'
  | 'moduleKnowledgeBase'
  | 'moduleMaterials'
  | 'moduleKnowledge'
  | 'modulePractice'
  | 'moduleReview'
  | 'moduleReport'
> = {
  overview: 'moduleOverview',
  knowledge_base: 'moduleKnowledgeBase',
  materials: 'moduleMaterials',
  knowledge: 'moduleKnowledge',
  practice: 'modulePractice',
  review: 'moduleReview',
  report: 'moduleReport',
}

// The learning dashboard: the study workspace (process sidebar, current module,
// inspector). A new session starts with a topic/materials form; the first stage
// turns that input into materials.json, then the remaining stages build the
// knowledge map, plan, notes, practice, and review. The host shell provides the
// header, the project switcher, and the chat sidebar; this component owns
// everything below them.
export function AssistantDashboard({ plugin, projectId }: PluginDashboardProps) {
  const { t, stageLabel, roleLabel, gateLabel, artifactLabel, statusLabel } = useAssistantI18n()
  const { send: sendChat, busy: chatBusy } = useChat()
  const [run, setRun] = useState<PluginRunInfo | null>(null)
  const [moduleId, setModuleId] = useState('overview')
  const [processOpen, setProcessOpen] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)

  const pid = projectId
  const pluginId = plugin.id

  const loadRun = useCallback(async (project: string) => {
    if (!project) {
      setRun(null)
      setLoading(false)
      return
    }
    setLoading(true)
    try {
      const res = await listPluginRuns(project, pluginId)
      const learningRuns = res.runs.filter(isLearningRun)
      const latest = learningRuns.length
        ? [...learningRuns].sort((a, b) =>
            b.updated_at.localeCompare(a.updated_at),
          )[0]
        : null
      setRun(latest)
    } finally {
      setLoading(false)
    }
  }, [pluginId])

  useEffect(() => {
    loadRun(pid).catch((e: unknown) => setError(String(e)))
  }, [pid, loadRun])

  useEffect(() => {
    if (!pid || busy) return
    const id = window.setInterval(() => {
      listPluginRuns(pid, pluginId)
        .then((res) => {
          const learningRuns = res.runs.filter(isLearningRun)
          if (learningRuns.length === 0) return
          const latest = [...learningRuns].sort((a, b) =>
            b.updated_at.localeCompare(a.updated_at),
          )[0]
          if (
            !run ||
            latest.id !== run.id ||
            latest.updated_at !== run.updated_at
          ) {
            setRun(latest)
          }
        })
        .catch(() => {})
    }, 4000)
    return () => window.clearInterval(id)
  }, [pid, pluginId, busy, run])

  const act = async (kind: 'start' | 'approve') => {
    if (!run) return
    setBusy(true)
    setError(null)
    try {
      const next =
        kind === 'start'
          ? await startPluginRun(pid, pluginId, run.id)
          : await approvePluginRun(pid, pluginId, run.id)
      setRun(next)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const runStage = async (stageId: string) => {
    if (!run) return
    setBusy(true)
    setError(null)
    try {
      setRun(await startPluginRun(pid, pluginId, run.id, { stage: stageId }))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const reject = async (reason: string) => {
    if (!run) return
    setBusy(true)
    setError(null)
    try {
      setRun(await rejectPluginRun(pid, pluginId, run.id, { reason }))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const openModule = (id: string) => {
    setModuleId(id)
  }

  if (loading) {
    return (
      <main className={styles.workspace}>
        <div className={styles.empty}>
          <h2>{t('preparing')}</h2>
        </div>
      </main>
    )
  }

  if (!run) {
    return (
      <main className={styles.workspace}>
        <div className={styles.empty}>
          <h2>{t('chatStartTitle')}</h2>
          <p className={styles.muted}>{t('chatStartHint')}</p>
          <p className={styles.muted}>{t('chatStartExample')}</p>
          {error && <div className={styles.error}>{error}</div>}
          <button
            type="button"
            className={styles.primary}
            disabled={chatBusy}
            onClick={() => void sendChat(t('newRoundPrompt'))}
          >
            {t('newRound')}
          </button>
        </div>
      </main>
    )
  }

  const workflow = plugin.workflows?.[0] ?? null
  const completed = new Set(run.completed_stages ?? [])
  const total = workflow?.stages?.length ?? 0
  const done = completed.size
  const currentStage = workflow?.stages?.find((stage) => stage.id === run.stage) ?? null
  const module = moduleById(moduleId)

  const props = {
    projectId: pid,
    runId: run.id,
    moduleId: module.id,
    plugin,
    workflow,
    run,
    onAct: act,
    onRunStage: runStage,
    onReject: reject,
    onOpenArtifact: (id: string) => openModule(id),
  }

  return (
    <div className={styles.dashboard}>
      <main className={styles.workspaceShell}>
        <aside className={styles.railPanel}>
          <div className={styles.railSection}>
            <button
              type="button"
              className={styles.railSectionToggle}
              aria-expanded={processOpen}
              aria-controls="assistant-process-modules"
              onClick={() => setProcessOpen((open) => !open)}
            >
              <span>{t('processModules')}</span>
              <ChevronDown
                size={14}
                strokeWidth={1.8}
                aria-hidden
                className={`${styles.railChevron} ${
                  processOpen ? styles.railChevronOpen : ''
                }`}
              />
            </button>
            <div
              id="assistant-process-modules"
              className={`${styles.stageNav} ${processOpen ? '' : styles.stageNavCollapsed}`}
              hidden={!processOpen}
            >
              {(workflow?.stages ?? []).map((stage) => {
                const state = stageState(stage, run)
                const stageModule = moduleForStage(stage.id)
                const active = module.id === stageModule.id
                return (
                  <button
                    key={stage.id}
                    type="button"
                    className={`${styles.stageNavItem} ${
                      state === 'done'
                        ? styles.stageNavDone
                        : state === 'current' || state === 'waiting'
                          ? styles.stageNavCurrent
                          : ''
                    } ${active ? styles.stageNavCurrent : ''}`}
                    onClick={() => openModule(stageModule.id)}
                  >
                    {(() => {
                      const Icon = STAGE_ICONS[stage.id] ?? FileText
                      return (
                        <span className={styles.menuIcon}>
                          <Icon size={14} strokeWidth={1.8} aria-hidden />
                        </span>
                      )
                    })()}
                    <span className={styles.stageNavName}>{stageLabel(stage.id)}</span>
                    <span className={styles.stageNavMeta}>
                      {state === 'done'
                        ? t('completed')
                        : state === 'waiting'
                          ? t('waiting')
                          : state === 'current'
                            ? t('running')
                            : t('pending')}
                    </span>
                  </button>
                )
              })}
            </div>
          </div>

          <div className={`${styles.railSection} ${styles.workbenchSection}`}>
            <div className={styles.railSectionTitle}>{t('workbench')}</div>
            {workbenchModules().map((item) => (
              <button
                key={item.id}
                type="button"
                className={`${styles.utilityItem} ${module.id === item.id ? styles.utilityActive : ''}`}
                onClick={() => openModule(item.id)}
              >
                {(() => {
                  const Icon = WORKBENCH_ICONS[item.id] ?? LayoutDashboard
                  return (
                    <span className={styles.menuIcon}>
                      <Icon size={14} strokeWidth={1.8} aria-hidden />
                    </span>
                  )
                })()}
                {MODULE_KEYS[item.id] ? t(MODULE_KEYS[item.id]) : item.title}
              </button>
            ))}
          </div>
        </aside>

        <section className={styles.workMain}>
          {error && <div className={styles.error}>{error}</div>}
          {run.error && <div className={styles.error}>{run.error}</div>}

          <div className={styles.productionHead}>
            <div>
              <div className={styles.titleRow}>
                <h1>{t('brand')}</h1>
              </div>
              <div className={styles.sub}>
                {done}/{total} {t('flowStages')} · {total ? Math.round((done / total) * 100) : 0}% ·{' '}
                {new Date(run.updated_at).toLocaleString()}
              </div>
            </div>
            <div className={styles.actions}>
              <button
                type="button"
                className={styles.btn}
                disabled={chatBusy}
                onClick={() => void sendChat(t('newRoundPrompt'))}
              >
                {t('newRound')}
              </button>
              <Badge status={run.status} />
              {currentStage && (
                <button
                  type="button"
                  className={styles.primary}
                  onClick={() => openModule(moduleForStage(currentStage.id).id)}
                >
                  {run.status === 'waiting_gate' ? t('review') : t('continue')}
                </button>
              )}
            </div>
          </div>

          <div className={styles.stagebar}>
            {(workflow?.stages ?? []).map((stage) => {
              const state = stageState(stage, run)
              return (
                <span
                  key={stage.id}
                  className={`${styles.stagechip} ${
                    state === 'done'
                      ? styles.stagechipDone
                      : state === 'current' || state === 'waiting'
                        ? styles.stagechipCurrent
                        : ''
                  }`}
                >
                  <span className={styles.dot} />
                  {stageLabel(stage.id)}
                </span>
              )
            })}
          </div>

          <module.component {...props} />
        </section>

        <aside className={styles.inspector}>
          <div className={styles.inspectorTitle}>{t('currentStage')}</div>
          {currentStage ? (
            <>
              <div className={styles.inspectorStage}>{stageLabel(currentStage.id)}</div>
              <div className={styles.inspectorMeta}>
                {t('role')}: {roleLabel(currentStage.role)}
              </div>
              <div className={styles.inspectorMeta}>
                {t('gate')}: {currentStage.gate ? gateLabel(currentStage.gate) : '—'}
              </div>
              <div className={styles.inspectorMeta}>
                {t('status')}: {statusLabel(run.status)}
              </div>
            </>
          ) : (
            <div className={styles.inspectorMeta}>
              {run.status === 'completed' ? t('completed') : t('statusCreated')}
            </div>
          )}

          <div className={styles.inspectorActions}>
            {['created', 'failed', 'cancelled'].includes(run.status) && (
              <button
                type="button"
                className={styles.primary}
                disabled={busy}
                onClick={() => void (module.stageId ? runStage(module.stageId) : act('start'))}
              >
                {t('start')}
              </button>
            )}
            {run.status === 'waiting_gate' && (
              <button
                type="button"
                className={styles.primary}
                disabled={busy}
                onClick={() => void act('approve')}
              >
                {t('approveContinue')}
              </button>
            )}
          </div>

          <div className={styles.inspectorTitle}>{t('artifacts')}</div>
          <div className={styles.inspectorArtifacts}>
            {(run.artifacts ?? []).map((artifact) => (
              <button
                key={artifact.id}
                type="button"
                className={styles.inspectorArtifact}
                onClick={() => openModule(artifact.id)}
              >
                <span>{artifactLabel(artifact.id)}</span>
                <span className={styles.mono}>{artifact.status}</span>
              </button>
            ))}
          </div>
        </aside>
      </main>
    </div>
  )
}
