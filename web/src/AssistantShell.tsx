import { useState } from 'react'
import { HomeTopbar, MoreDialog, NewProjectDialog, useChat, useI18n } from '@agentzoo/web-ui'
import { AssistantDashboard } from './AssistantDashboard'
import styles from './assistant-shell.module.css'

// The conversation shell for the learning assistant: the shared home topbar
// (project switcher, brand, actions) above the assistant dashboard. Rendered
// at `/project/:projectId` and `/project/:projectId/topic/:topicId`; the URL
// is the source of truth for the active project.
export function AssistantShell() {
  const { projectId, createProject } = useChat()
  const { t } = useI18n()
  const [showMore, setShowMore] = useState(false)
  const [showNewProject, setShowNewProject] = useState(false)
  return (
    <div className={styles.shell}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      {projectId ? (
        <AssistantDashboard projectId={projectId} />
      ) : (
        <div className={styles.empty}>
          {t('home.noProjectHint')}
        </div>
      )}
      {showMore && <MoreDialog onClose={() => setShowMore(false)} />}
      {showNewProject && (
        <NewProjectDialog
          onClose={() => setShowNewProject(false)}
          onCreate={async (name, path) => {
            await createProject(name, path)
            setShowNewProject(false)
          }}
        />
      )}
    </div>
  )
}
