import type { PluginInfo } from '@agentzoo/web-ui'

export interface PluginDashboardProps {
  plugin: PluginInfo
  projectId: string
  onProjectChange: (id: string) => void
}
