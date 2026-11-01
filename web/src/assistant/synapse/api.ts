export interface SynapseConfigSummary {
  configured: boolean
  gateway_url: string
  tenant_id: string
  auth: string
}

export interface SynapseSession {
  logged_in: boolean
  tenant_id: string
  user_account: string
  name: string
  expires_in: number
}

export interface SynapsePaperRecord {
  paper_id: number
  title: string
  question_count: number
  question_ids: number[]
  duration_minutes: number
  synced_at: string
}

export interface SynapseKnowledgePointRecord {
  id: number
  name: string | null
  synced_at: string | null
}

export interface SynapseRecord {
  paper_id: number
  title: string
  question_count: number
  question_ids: number[]
  knowledge_point_id: number
  knowledge_name?: string
  knowledge_point_count: number
  duration_minutes: number
  synced_at: string
  tenant_id: string
  gateway_url: string
  run_id: string
  papers?: SynapsePaperRecord[]
  knowledge_points?: SynapseKnowledgePointRecord[]
}

export type SynapsePublishMode = 'add' | 'overwrite'

export interface SynapseStatus {
  config: SynapseConfigSummary
  session: SynapseSession | null
  can_publish: boolean
  record: SynapseRecord | null
}

export interface SynapsePublishResult {
  record: SynapseRecord
  already_synced: boolean
}

export interface SynapseLoginRequest {
  tenant_id?: string
  user_account: string
  password: string
}

const base = (projectId: string, pluginId: string, runId: string) =>
  `/projects/${encodeURIComponent(projectId)}/apps/${encodeURIComponent(pluginId)}/runs/${encodeURIComponent(runId)}/synapse`

async function call<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init)
  const body = await res.json().catch(() => null)
  if (!res.ok || !body || body.code !== 'ok') {
    throw new Error(body?.message || `HTTP ${res.status}`)
  }
  return body.data as T
}

export function getSynapseStatus(
  projectId: string,
  pluginId: string,
  runId: string,
): Promise<SynapseStatus> {
  return call<SynapseStatus>(base(projectId, pluginId, runId))
}

export function publishToSynapse(
  projectId: string,
  pluginId: string,
  runId: string,
  mode: SynapsePublishMode = 'add',
): Promise<SynapsePublishResult> {
  return call<SynapsePublishResult>(`${base(projectId, pluginId, runId)}/publish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mode }),
  })
}

export function loginStudio(
  projectId: string,
  pluginId: string,
  runId: string,
  request: SynapseLoginRequest,
): Promise<{ session: SynapseSession }> {
  return call<{ session: SynapseSession }>(
    `${base(projectId, pluginId, runId)}/login`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    },
  )
}

export function logoutStudio(
  projectId: string,
  pluginId: string,
  runId: string,
): Promise<{ session: null }> {
  return call<{ session: null }>(`${base(projectId, pluginId, runId)}/logout`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: '{}',
  })
}
