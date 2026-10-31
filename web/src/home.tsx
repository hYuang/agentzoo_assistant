import { useEffect, useState } from 'react'
import { listPlugins } from '@agentzoo/web-ui/api/api'
import { MoreDialog } from '@agentzoo/web-ui/components/common/MoreDialog/MoreDialog'
import { NewProjectDialog } from '@agentzoo/web-ui/components/business/NewProjectDialog/NewProjectDialog'
import { HomeTopbar } from '@agentzoo/web-ui/pages/home/HomeTopbar'
import { useChat } from '@agentzoo/web-ui/store/chatContext'
import { useI18n } from '@agentzoo/web-ui/i18n'
import type { PluginInfo } from '@agentzoo/web-ui/types'

export function AssistantHome() {
  const { createProject, projectId } = useChat()
  const { t } = useI18n()
  const [plugin, setPlugin] = useState<PluginInfo | null>(null)
  const [showNewProject, setShowNewProject] = useState(false)
  const [showMore, setShowMore] = useState(false)

  useEffect(() => {
    listPlugins()
      .then((res) =>
        setPlugin(res.plugins.find((item) => item.id === 'assistant') ?? null),
      )
      .catch(() => setPlugin(null))
  }, [])

  const page =
    plugin?.ui?.find((item) => item.slot === 'plugin.dashboard')?.page ??
    'index.html'

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      {projectId && plugin ? (
        <iframe
          title={plugin.name}
          src={`/plugins/assistant/ui/${page}?projectId=${encodeURIComponent(projectId)}`}
          style={{ border: 0, width: '100%', height: '100%' }}
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
