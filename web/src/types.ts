export interface ArtifactSpec {
  id: string
  title: string
  path?: string | null
  version?: number
}

export interface RoleInfo {
  id: string
  name: string
  kind: 'human' | 'agent'
  capabilities: string[]
  can_approve: string[]
}

export interface GateInfo {
  id: string
  name: string
  approver: string
  on_reject?: string | null
}

export interface StageInfo {
  id: string
  name: string
  role: string
  requires: string[]
  depends_on: string[]
  inputs: string[]
  outputs: string[]
  gate?: string | null
  foreach?: string | null
}

export interface WorkflowInfo {
  id: string
  name: string
  roles: RoleInfo[]
  gates: GateInfo[]
  stages: StageInfo[]
}

export interface AssistantMeta {
  name: string
  workflow: WorkflowInfo
  artifacts: ArtifactSpec[]
}

export interface RunArtifactInfo {
  id: string
  title: string
  path: string
  status: string
  size: number
  updated_at?: string | null
}

export interface RunInfo {
  id: string
  project_id: string
  workflow_id?: string | null
  stage?: string | null
  session_id?: string | null
  completed_stages: string[]
  status: string
  input: Record<string, unknown>
  error?: string | null
  review_notes?: string | null
  created_at: string
  updated_at: string
  artifacts: RunArtifactInfo[]
}

export interface ArtifactContent {
  content: string
  encoding: string
}

export interface UploadResponse {
  path: string
  name: string
  size: number
}

export interface StartRunRequest {
  model?: string | null
  stage?: string | null
}

export interface CreateDirectoryResponse {
  path: string
}
