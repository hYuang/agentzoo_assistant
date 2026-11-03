import { ApiError } from '@agentzoo/web-ui'

import type {
  ArtifactContent,
  AssistantMeta,
  CreateDirectoryResponse,
  RunInfo,
  StartRunRequest,
  UploadResponse,
} from './types'

const base = (projectId: string) =>
  `/projects/${encodeURIComponent(projectId)}/assistant`

interface Envelope<T> {
  code: string
  message: string
  data: T
}

async function unwrap<T>(res: Response): Promise<T> {
  const body = (await res.json().catch(() => null)) as Envelope<T> | null
  if (!res.ok || !body || body.code !== 'ok') {
    throw new ApiError(
      body?.code ?? `http_${res.status}`,
      body?.message || `HTTP ${res.status}`,
    )
  }
  return body.data
}

async function get<T>(path: string): Promise<T> {
  return unwrap<T>(await fetch(path))
}

async function post<T>(path: string, body: unknown = {}): Promise<T> {
  return unwrap<T>(
    await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }),
  )
}

export function getMeta(projectId: string): Promise<AssistantMeta> {
  return get<AssistantMeta>(base(projectId))
}

export function listRuns(projectId: string): Promise<RunInfo[]> {
  return get<{ runs: RunInfo[] }>(`${base(projectId)}/runs`).then(
    (res) => res.runs,
  )
}

export function getRun(projectId: string, runId: string): Promise<RunInfo> {
  return get<RunInfo>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}`,
  )
}

export function createRun(
  projectId: string,
  input: Record<string, unknown>,
): Promise<RunInfo> {
  return post<RunInfo>(`${base(projectId)}/runs`, { input })
}

export function startRun(
  projectId: string,
  runId: string,
  request: StartRunRequest = {},
): Promise<RunInfo> {
  return post<RunInfo>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/start`,
    request,
  )
}

export function approveRun(
  projectId: string,
  runId: string,
): Promise<RunInfo> {
  return post<RunInfo>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/approve`,
  )
}

export function rejectRun(
  projectId: string,
  runId: string,
  reason: string,
): Promise<RunInfo> {
  return post<RunInfo>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/reject`,
    { reason },
  )
}

export function readArtifact(
  projectId: string,
  runId: string,
  artifactId: string,
): Promise<ArtifactContent> {
  return get<ArtifactContent>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/artifacts/${encodeURIComponent(artifactId)}`,
  )
}

export async function writeArtifact(
  projectId: string,
  runId: string,
  artifactId: string,
  body: { content: string; encoding?: string | null },
): Promise<RunInfo> {
  return unwrap<RunInfo>(
    await fetch(
      `${base(projectId)}/runs/${encodeURIComponent(runId)}/artifacts/${encodeURIComponent(artifactId)}`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ),
  )
}

export function uploadFile(
  projectId: string,
  runId: string,
  body: { name: string; data: string; dir?: string | null },
): Promise<UploadResponse> {
  return post<UploadResponse>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/uploads`,
    body,
  )
}

export function createDirectory(
  projectId: string,
  runId: string,
  path: string,
): Promise<CreateDirectoryResponse> {
  return post<CreateDirectoryResponse>(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/directories`,
    { path },
  )
}

export async function deleteUpload(
  projectId: string,
  runId: string,
  rel: string,
): Promise<void> {
  const segments = rel.split('/').filter(Boolean).map(encodeURIComponent)
  const res = await fetch(
    `${base(projectId)}/runs/${encodeURIComponent(runId)}/uploads/${segments.join('/')}`,
    { method: 'DELETE' },
  )
  if (!res.ok) throw await unwrap<unknown>(res)
}

export function uploadUrl(
  projectId: string,
  runId: string,
  rel: string,
): string {
  const segments = rel.split('/').filter(Boolean).map(encodeURIComponent)
  return `${base(projectId)}/runs/${encodeURIComponent(runId)}/uploads/${segments.join('/')}`
}
