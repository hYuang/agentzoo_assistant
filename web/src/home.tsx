import { useEffect, useState } from 'react'
import {
  HomeTopbar,
  MoreDialog,
  NewProjectDialog,
  listPlugins,
  useChat,
  useI18n,
  type PluginInfo,
} from '@agentzoo/web-ui'
import { AssistantDashboard } from './assistant/AssistantDashboard'
import { ASSISTANT_PLUGIN_ID } from './assistant/constants'

export function AssistantHome() {
  const { createProject, projectId, setActiveProject } = useChat()
  const { t } = useI18n()
  const [plugin, setPlugin] = useState<PluginInfo | null>(null)
  const [showNewProject, setShowNewProject] = useState(false)
  const [showMore, setShowMore] = useState(false)

  useEffect(() => {
    listPlugins()
      .then((res) =>
        setPlugin(
          res.plugins.find((item) => item.id === ASSISTANT_PLUGIN_ID) ?? null,
        ),
      )
      .catch(() => setPlugin(null))
  }, [])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      {projectId && plugin ? (
        <AssistantDashboard
          plugin={plugin}
          projectId={projectId}
          onProjectChange={setActiveProject}
        />
      ) : (
        <div style={{ margin: '12vh auto', color: 'var(--fg-muted)' }}>
          {t('home.noProjectHint')}
        </div>
      )}
      {showNewProject && (
        <NewProjectDialog
          onClose={() => setShowNewProject(false)}
          onCreate={async (name, path) => {
            await createProject(name, path)
            setShowNewProject(false)
          }}
        />
      )}
      {showMore && <MoreDialog onClose={() => setShowMore(false)} />}
    </div>
  )
}
