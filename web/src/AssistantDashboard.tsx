import { useCallback, useEffect, useState } from 'react'
import {
  BookOpen,
  Check,
  ChevronDown,
  FileText,
  FolderOpen,
  LayoutDashboard,
  Library,
  ListChecks,
  Network,
  RotateCcw,
  Share2,
  type LucideIcon,
} from 'lucide-react'
import { useChat } from '@agentzoo/web-ui'
import type { AssistantDashboardProps } from './contract'
import type { AssistantMeta, RunInfo, WorkflowInfo } from './types'
import { approveRun, getMeta, listRuns, rejectRun, startRun } from './api'
import { Badge, stageState } from './modules/shared'
import { moduleById, moduleForStage, workbenchModules } from './modules/registry'
import { StudioSessionProvider } from './synapse/session'
import { StudioAccountPanel } from './synapse/account'
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
  synapse: Share2,
  materials: FolderOpen,
  knowledge: Network,
  practice: ListChecks,
  review: RotateCcw,
  report: FileText,
}

function latestRun(runs: RunInfo[]): RunInfo | null {
  if (runs.length === 0) return null
  return [...runs].sort((a, b) => b.updated_at.localeCompare(a.updated_at))[0]
}

const MODULE_KEYS: Record<
  string,
  | 'moduleOverview'
  | 'moduleKnowledgeBase'
  | 'moduleSynapse'
  | 'moduleMaterials'
  | 'moduleKnowledge'
  | 'modulePractice'
  | 'moduleReview'
  | 'moduleReport'
> = {
  overview: 'moduleOverview',
  knowledge_base: 'moduleKnowledgeBase',
  synapse: 'moduleSynapse',
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
export function AssistantDashboard({ projectId }: AssistantDashboardProps) {
  const { t, stageLabel, roleLabel, gateLabel, artifactLabel, statusLabel } = useAssistantI18n()
  const { send: sendChat, busy: chatBusy } = useChat()
  const [meta, setMeta] = useState<AssistantMeta | null>(null)
  const [run, setRun] = useState<RunInfo | null>(null)
  const [moduleId, setModuleId] = useState('overview')
  const [processOpen, setProcessOpen] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(true)

  const pid = projectId

  const loadRun = useCallback(async (project: string) => {
    if (!project) {
      setRun(null)
      setLoading(false)
      return
    }
    setLoading(true)
    try {
      setRun(latestRun(await listRuns(project)))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    getMeta(pid)
      .then((next) => {
        if (!cancelled) setMeta(next)
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e))
      })
    return () => {
      cancelled = true
    }
  }, [pid])

  useEffect(() => {
    loadRun(pid).catch((e: unknown) => setError(String(e)))
  }, [pid, loadRun])

  useEffect(() => {
    if (!pid || busy) return
    const id = window.setInterval(() => {
      listRuns(pid)
        .then((runs) => {
          const latest = latestRun(runs)
          if (!latest) return
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
  }, [pid, busy, run])

  const act = async (kind: 'start' | 'approve') => {
    if (!run) return
    setBusy(true)
    setError(null)
    try {
      const next =
        kind === 'start'
          ? await startRun(pid, run.id)
          : await approveRun(pid, run.id)
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
      setRun(await startRun(pid, run.id, { stage: stageId }))
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
      setRun(await rejectRun(pid, run.id, reason))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  const openModule = (id: string) => {
    setModuleId(id)
  }

  if (loading || !meta) {
    return (
      <main className={styles.workspace}>
        <div className={styles.empty}>
          <h2>{t('preparing')}</h2>
          {error && <div className={styles.error}>{error}</div>}
        </div>
      </main>
    )
  }

  const workflow: WorkflowInfo = meta.workflow
  const activeRun: RunInfo =
    run ?? {
      id: '',
      project_id: pid,
      workflow_id: workflow.id,
      stage: null,
      session_id: null,
      completed_stages: [],
      status: 'created',
      input: {},
      created_at: '',
      updated_at: '',
      artifacts: [],
    }
  const completed = new Set(activeRun.completed_stages ?? [])
  const total = workflow.stages.length
  const done = completed.size
  const currentStage =
    workflow.stages.find((stage) => stage.id === activeRun.stage) ?? null
  const module = moduleById(moduleId)

  const props = {
    projectId: pid,
    runId: activeRun.id,
    moduleId: module.id,
    workflow,
    run: activeRun,
    onAct: act,
    onRunStage: runStage,
    onReject: reject,
    onOpenArtifact: (id: string) => openModule(id),
  }

  return (
    <StudioSessionProvider projectId={pid} runId={activeRun.id}>
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
                  const state = stageState(stage, activeRun)
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

            <StudioAccountPanel />
          </aside>

          <section className={styles.workMain}>
            {error && <div className={styles.error}>{error}</div>}
            {run?.error && <div className={styles.error}>{run?.error}</div>}

            <div className={styles.productionHead}>
              <div>
                <div className={styles.titleRow}>
                  <h1>{t('brand')}</h1>
                </div>
                <div className={styles.sub}>
                  {done}/{total} {t('flowStages')} ·{' '}
                  {total ? Math.round((done / total) * 100) : 0}%
                  {run ? ` · ${new Date(run.updated_at).toLocaleString()}` : null}
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
                <Badge status={activeRun.status} />
                {run &&
                  ['created', 'failed', 'cancelled'].includes(run.status) && (
                  <button
                    type="button"
                    className={styles.primary}
                    disabled={busy}
                    onClick={() => void act('start')}
                  >
                    {t('start')}
                  </button>
                )}
                {run?.status === 'waiting_gate' && (
                  <button
                    type="button"
                    className={styles.primary}
                    disabled={busy}
                    onClick={() => void act('approve')}
                  >
                    {t('approveContinue')}
                  </button>
                )}
                {currentStage && (
                  <button
                    type="button"
                    className={styles.primary}
                    onClick={() => openModule(moduleForStage(currentStage.id).id)}
                  >
                    {run?.status === 'waiting_gate' ? t('review') : t('continue')}
                  </button>
                )}
              </div>
            </div>

            <div className={styles.stagebar}>
              {(workflow?.stages ?? []).map((stage) => {
                const state = stageState(stage, activeRun)
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

            {run ? (
              <module.component {...props} />
            ) : (
              <div className={styles.empty}>
                <h2>{t('chatStartTitle')}</h2>
                <p className={styles.muted}>{t('chatStartHint')}</p>
                <p className={styles.muted}>{t('chatStartExample')}</p>
                <button
                  type="button"
                  className={styles.primary}
                  disabled={chatBusy}
                  onClick={() => void sendChat(t('newRoundPrompt'))}
                >
                  {t('newRound')}
                </button>
              </div>
            )}
          </section>

          <aside className={styles.inspector}>
            <div className={styles.inspectorTitle}>{t('processModules')}</div>
            <div className={styles.stageFlow}>
              {(workflow?.stages ?? []).map((stage, index, stages) => {
                const state = stageState(stage, activeRun)
                const stageModule = moduleForStage(stage.id)
                const active = module.id === stageModule.id
                const Icon = STAGE_ICONS[stage.id] ?? FileText
                return (
                  <div key={stage.id} className={styles.flowStep}>
                    <button
                      type="button"
                      className={`${styles.flowNode} ${
                        state === 'done'
                          ? styles.flowNodeDone
                          : state === 'current' || state === 'waiting'
                            ? styles.flowNodeCurrent
                            : ''
                      } ${active ? styles.flowNodeActive : ''}`}
                      onClick={() => openModule(stageModule.id)}
                    >
                      <span className={styles.flowIcon}>
                        {state === 'done' ? (
                          <Check size={13} strokeWidth={2.2} aria-hidden />
                        ) : (
                          <Icon size={14} strokeWidth={1.8} aria-hidden />
                        )}
                      </span>
                      <span className={styles.flowBody}>
                        <span className={styles.flowName}>{stageLabel(stage.id)}</span>
                        <span className={styles.flowMeta}>
                          {state === 'done'
                            ? t('completed')
                            : state === 'waiting'
                              ? t('waiting')
                              : state === 'current'
                                ? t('running')
                                : t('pending')}
                        </span>
                      </span>
                    </button>
                    {index < stages.length - 1 && (
                      <span
                        className={`${styles.flowConnector} ${
                          state === 'done' ? styles.flowConnectorDone : ''
                        }`}
                        aria-hidden
                      />
                    )}
                  </div>
                )
              })}
            </div>

          </aside>
        </main>
      </div>
    </StudioSessionProvider>
  )
}
