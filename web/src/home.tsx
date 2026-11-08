import { useState } from 'react'
import { HomeTopbar, MoreDialog, NewProjectDialog, useChat } from '@agentzoo/web-ui'
import { AssistantDashboard } from './AssistantDashboard'
import styles from './home.module.css'

// The root page: the assistant dashboard directly. The topbar's project
// switcher handles project selection and creation; the dashboard renders
// inline below it.
export function AssistantHome() {
  const { projectId, createProject } = useChat()
  const [showNewProject, setShowNewProject] = useState(false)
  const [showMore, setShowMore] = useState(false)

  return (
    <div className={styles.home}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      <div className={styles.body}>
        {projectId ? (
          <AssistantDashboard projectId={projectId} />
        ) : (
          <div className={styles.empty}>
            <p className={styles.emptyText}>
              Select or create a project to get started.
            </p>
            <button
              type="button"
              className={styles.emptyBtn}
              onClick={() => setShowNewProject(true)}
            >
              Create a project
            </button>
          </div>
        )}
      </div>
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
