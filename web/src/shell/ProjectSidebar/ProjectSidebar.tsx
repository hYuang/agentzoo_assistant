import { useEffect, useRef, useState } from 'react'
import { Pencil, Trash2 } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import {
  formatDateTime,
  useChat,
  useI18n,
  type MessageKey,
  type PanelCtx,
} from '@agentzoo/web-ui'
import styles from './project-sidebar.module.css'

// The session's display status: the active session reflects the live runtime
// status (error / busy / connecting), other sessions are only known to be
// running when this tab holds an in-flight turn for them.
type SessionStatus = 'idle' | 'busy' | 'error' | 'connecting'
const STATUS_KEY: Record<SessionStatus, MessageKey> = {
  idle: 'sidebar.status.idle',
  busy: 'sidebar.status.busy',
  error: 'sidebar.status.error',
  connecting: 'sidebar.status.connecting',
}

// The conversation page's sidebar: the current project's recent conversations
// (click to switch), with the project name as a header back to the home page.
// Collapsed, it shrinks to a rail of project initials for quick switching.
// The layout engine owns its width and resize handle; collapse is requested
// through `ctx`.
export function ProjectSidebar({ ctx }: { ctx: PanelCtx }) {
  const {
    projects,
    projectId,
    sessions,
    sessionId,
    selectProject,
    newSession,
    openSession,
    deleteSession,
    renameConversation,
    status,
    busySessions,
    connected,
  } = useChat()
  const { t } = useI18n()
  const navigate = useNavigate()
  const collapsed = ctx.collapsed

  const project = projects.find((p) => p.id === projectId) ?? null

  // One conversation groups every session created for a chat. The sidebar
  // lists the conversation's main session only; dispatched sessions are peers
  // and are merged into the same transcript page.
  const byId = new Map(sessions.map((s) => [s.session_id, s]))
  const activeConversation = sessionId
    ? (byId.get(sessionId)?.conversation_id ?? null)
    : null
  // A conversation is busy while any of its sessions has an in-flight turn
  // (the coordinator, or a routed specialist running on its behalf).
  const conversationBusy = [...busySessions].some(
    (sid) => byId.get(sid)?.conversation_id === activeConversation,
  )

  const statusOf = (sid: string): SessionStatus => {
    const conversation = byId.get(sid)?.conversation_id === activeConversation
    if (conversation) {
      if (status === 'error') return 'error'
      if (status === 'busy' || conversationBusy) return 'busy'
      if (status === 'connecting' || !connected) return 'connecting'
      return 'idle'
    }
    return busySessions.has(sid) ? 'busy' : 'idle'
  }

  const visibleSessions = sessions.filter(
    (s) =>
      s.is_main &&
      (s.message_count > 0 || s.conversation_id === activeConversation),
  )

  // Inline rename of a conversation's title (the group-chat name).
  const [editingCid, setEditingCid] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  useEffect(() => {
    if (editingCid) inputRef.current?.focus()
  }, [editingCid])
  const startEdit = (s: (typeof visibleSessions)[number]) => {
    setDraft(s.conversation_title ?? '')
    setEditingCid(s.conversation_id)
  }
  const commitEdit = async () => {
    const cid = editingCid
    setEditingCid(null)
    if (!cid) return
    const next = draft.trim()
    if (!next) return
    await renameConversation(cid, next)
  }

  const toggle = ctx.toggleCollapse

  return (
    <aside
      className={
        collapsed
          ? `${styles.sidebar} ${styles.sidebarCollapsed}`
          : styles.sidebar
      }
    >
      <div className={styles.head}>
        {!collapsed && (
          <button
            className={styles.projectHead}
            onClick={() => navigate('/')}
            title={
              project
                ? t('home.projectTitle', {
                    name: project.name,
                    path: project.path,
                  })
                : t('sidebar.title')
            }
            aria-label={t('header.home')}
          >
            {project?.name ?? t('sidebar.title')}
          </button>
        )}
        <div className={styles.headBtns}>
          <button
            className={styles.newBtn}
            onClick={newSession}
            title={t('header.newChat')}
            aria-label={t('header.newChat')}
          >
            +
          </button>
          <button
            className={styles.collapseBtn}
            onClick={toggle}
            title={collapsed ? t('sidebar.expand') : t('sidebar.collapse')}
            aria-label={collapsed ? t('sidebar.expand') : t('sidebar.collapse')}
          >
            {collapsed ? '»' : '«'}
          </button>
        </div>
      </div>
      <div className={styles.list}>
        {collapsed
          ? projects.map((p) => (
              <button
                key={p.id}
                className={
                  p.id === projectId
                    ? styles.itemCollapsedActive
                    : styles.itemCollapsed
                }
                onClick={() => void selectProject(p.id)}
                title={t('sidebar.itemTitle', { name: p.name, path: p.path })}
                aria-label={t('sidebar.select', { name: p.name })}
              >
                {p.name.charAt(0).toUpperCase() || '?'}
              </button>
            ))
          : visibleSessions.map((s) => {
              const st = statusOf(s.session_id)
              const title = s.conversation_title?.trim()
              const name = title || s.preview || t('home.recentNoPreview')
              return (
                <div
                  key={s.session_id}
                  className={
                    s.conversation_id === activeConversation
                      ? styles.itemActive
                      : styles.item
                  }
                  onClick={() =>
                    projectId && openSession(projectId, s.session_id)
                  }
                  title={name}
                >
                  <div className={styles.itemMain}>
                    <div className={styles.itemTop}>
                      {editingCid === s.conversation_id ? (
                        <input
                          ref={inputRef}
                          className={styles.titleInput}
                          value={draft}
                          onChange={(e) => setDraft(e.target.value)}
                          onClick={(e) => e.stopPropagation()}
                          onBlur={() => void commitEdit()}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') void commitEdit()
                            if (e.key === 'Escape') setEditingCid(null)
                          }}
                          aria-label={t('member.renameTitle')}
                        />
                      ) : (
                        <>
                          <span className={styles.itemName}>{name}</span>
                          <button
                            className={styles.renameBtn}
                            title={t('member.renameTitle')}
                            aria-label={t('member.renameTitle')}
                            onClick={(e) => {
                              e.stopPropagation()
                              startEdit(s)
                            }}
                          >
                            <Pencil size={12} />
                          </button>
                        </>
                      )}
                    </div>
                    <div className={styles.itemMeta}>
                      <span className={styles.itemTime}>
                        {formatDateTime(s.last_activity_at)}
                      </span>
                      <span className={styles.status} data-status={st}>
                        {t(STATUS_KEY[st])}
                      </span>
                    </div>
                  </div>
                  <button
                    className={styles.delBtn}
                    title={t('home.deleteSession')}
                    aria-label={t('home.deleteSession')}
                    onClick={(e) => {
                      e.stopPropagation()
                      if (projectId)
                        void deleteSession(projectId, s.conversation_id)
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              )
            })}
        {!collapsed && visibleSessions.length === 0 && (
          <div className={styles.empty}>{t('sidebar.emptySessions')}</div>
        )}
      </div>
    </aside>
  )
}
