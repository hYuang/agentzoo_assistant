import { useState } from 'react'
import {
  HomeTopbar,
  MoreDialog,
  NewProjectDialog,
  useChat,
  useI18n,
} from '@agentzoo/web-ui'
import { AssistantDashboard } from './AssistantDashboard'

export function AssistantHome() {
  const { createProject, projectId } = useChat()
  const { t } = useI18n()
  const [showNewProject, setShowNewProject] = useState(false)
  const [showMore, setShowMore] = useState(false)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      {projectId ? (
        <AssistantDashboard projectId={projectId} />
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
