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

export async function loadLearnerState(
  projectId: string,
  runId: string,
): Promise<LearnerState> {
  try {
    const res = await readPluginArtifact(
      projectId,
      'assistant',
      runId,
      'learner_state',
    )
    if (res.encoding !== 'utf-8') return {}
    return (safeJson(res.content) as LearnerState) ?? {}
  } catch {
    return {}
  }
}

export async function saveLearnerState(
  projectId: string,
  runId: string,
  state: LearnerState,
): Promise<void> {
  await writePluginArtifact(projectId, 'assistant', runId, 'learner_state', {
    content: JSON.stringify(state, null, 2),
    encoding: 'utf-8',
  })
}

// Merge a patch into the stored state so the quiz, plan, and flashcard
// progress do not clobber one another.
export async function updateLearnerState(
  projectId: string,
  runId: string,
  patch: Partial<LearnerState>,
): Promise<LearnerState> {
  const current = await loadLearnerState(projectId, runId)
  const next = { ...current, ...patch }
  await saveLearnerState(projectId, runId, next)
  return next
}
