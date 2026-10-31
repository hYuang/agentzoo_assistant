import type { AssistantModule } from './types'
import { OverviewModule } from './overview'
import { MaterialsModule } from './materials'
import { KnowledgeBaseModule } from './knowledgeBase'
import { KnowledgeModule, ReportModule } from './markdown'
import { PracticeModule } from './practice'
import { ReviewModule } from './review'

const MODULES: AssistantModule[] = [
  {
    id: 'overview',
    title: 'Overview',
    group: 'workbench',
    order: 0,
    component: OverviewModule,
  },
  {
    id: 'knowledge_base',
    title: 'Knowledge base',
    group: 'workbench',
    order: 1,
    component: KnowledgeBaseModule,
  },
  {
    id: 'materials',
    title: 'Materials',
    group: 'process',
    order: 1,
    stageId: 'intake',
    component: MaterialsModule,
  },
  {
    id: 'knowledge',
    title: 'Knowledge points',
    group: 'process',
    order: 2,
    stageId: 'map',
    component: KnowledgeModule,
  },
  {
    id: 'practice',
    title: 'Timed practice',
    group: 'process',
    order: 3,
    stageId: 'practice',
    component: PracticeModule,
  },
  {
    id: 'review',
    title: 'Analysis',
    group: 'process',
    order: 4,
    stageId: 'review',
    component: ReviewModule,
  },
  {
    id: 'report',
    title: 'Report',
    group: 'process',
    order: 5,
    stageId: 'report',
    component: ReportModule,
  },
]

export function moduleById(id: string): AssistantModule {
  // The practice artifact and module have different ids (`quiz` vs
  // `practice`); artifact clicks must still land on the module.
  const moduleId = id === 'quiz' ? 'practice' : id
  return MODULES.find((module) => module.id === moduleId) ?? MODULES[0]
}

export function moduleForStage(stageId: string): AssistantModule {
  return MODULES.find((module) => module.stageId === stageId) ?? MODULES[0]
}

export function workbenchModules(): AssistantModule[] {
  return MODULES.filter((module) => module.group === 'workbench')
}
