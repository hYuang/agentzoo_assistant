import type { ComponentType } from 'react'
import type {
  PluginInfo,
  PluginRunInfo,
  PluginStageInfo,
  PluginWorkflowInfo,
} from '@agentzoo/web-ui'

export interface ModuleProps {
  projectId: string
  runId: string
  moduleId: string
  artifactId?: string
  plugin: PluginInfo | null
  workflow: PluginWorkflowInfo | null
  run: PluginRunInfo
  onAct: (kind: 'start' | 'approve') => void
  onRunStage: (stageId: string) => void
  onReject: (reason: string) => void
  onOpenArtifact: (id: string) => void
}

export interface AssistantModule {
  id: string
  title: string
  group: 'process' | 'workbench'
  order: number
  stageId?: string
  component: ComponentType<ModuleProps>
}

export type { PluginStageInfo, PluginWorkflowInfo }
