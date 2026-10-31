import { readPluginArtifact, writePluginArtifact } from '@agentzoo/web-ui'
import { safeJson } from './shared'

export type LearnerAnswer = {
  value: number | string
  correct?: boolean | null
}

export type LearnerState = {
  answers?: Record<string, LearnerAnswer>
  submitted?: boolean
  score?: number
  total?: number
  started_at?: string | null
  submitted_at?: string | null
  elapsed_seconds?: number
  time_limit_minutes?: number
  auto_submitted?: boolean
  plan_done?: string[]
  card_ratings?: Record<string, 'known' | 'unknown'>
}

const LEARNER_STATE_ARTIFACT = 'learner_state'

export async function loadLearnerState(
  projectId: string,
  pluginId: string,
  runId: string,
): Promise<LearnerState> {
  try {
    const res = await readPluginArtifact(
      projectId,
      pluginId,
      runId,
      LEARNER_STATE_ARTIFACT,
    )
    if (res.encoding !== 'utf-8') return {}
    return (safeJson(res.content) as LearnerState) ?? {}
  } catch {
    return {}
  }
}

export async function saveLearnerState(
  projectId: string,
  pluginId: string,
  runId: string,
  state: LearnerState,
): Promise<void> {
  await writePluginArtifact(projectId, pluginId, runId, LEARNER_STATE_ARTIFACT, {
    content: JSON.stringify(state, null, 2),
    encoding: 'utf-8',
  })
}

// Merge a patch into the stored state so the quiz, plan, and flashcard
// progress do not clobber one another.
export async function updateLearnerState(
  projectId: string,
  pluginId: string,
  runId: string,
  patch: Partial<LearnerState>,
): Promise<LearnerState> {
  const current = await loadLearnerState(projectId, pluginId, runId)
  const next = { ...current, ...patch }
  await saveLearnerState(projectId, pluginId, runId, next)
  return next
}
