import { useState } from 'react'
import {
  HomeTopbar,
  Layout,
  MoreDialog,
  NewProjectDialog,
  useChat,
  useI18n,
  useLayoutSpec,
  type LayoutNode,
} from '@agentzoo/web-ui'
import { AssistantDashboard } from './AssistantDashboard'
import styles from './assistant-shell.module.css'

const shellLayout: LayoutNode = {
  id: 'shell',
  dir: 'row',
  children: [
    {
      id: 'center',
      dir: 'column',
      grow: true,
      children: [
        { id: 'members', panel: 'members', locked: true },
        { id: 'transcript', panel: 'transcript', grow: true },
        { id: 'composer', panel: 'composer' },
      ],
    },
    {
      id: 'toolbox',
      panel: 'toolbox',
      resizable: 'right',
      collapsible: true,
      defaultCollapsed: true,
      collapsedSize: 48,
      size: 280,
      min: 200,
      max: 480,
    },
  ],
}

export function AssistantShell() {
  const { projectId, createProject } = useChat()
  const { t } = useI18n()
  const spec = useLayoutSpec('shell', shellLayout)
  const [showMore, setShowMore] = useState(false)
  const [showNewProject, setShowNewProject] = useState(false)
  return (
    <div className={styles.shell}>
      <HomeTopbar
        onNewProject={() => setShowNewProject(true)}
        onMore={() => setShowMore(true)}
      />
      {projectId ? (
        <div className={styles.body}>
          <AssistantDashboard projectId={projectId} />
          <Layout id="shell" spec={spec} className={styles.chat} />
        </div>
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
