import type { PluginInfo } from '@agentzoo/web-ui/types'

export interface PluginDashboardProps {
  plugin: PluginInfo
  projectId: string
  onProjectChange: (id: string) => void
}
