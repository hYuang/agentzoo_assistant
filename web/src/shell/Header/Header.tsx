import { useEffect, useMemo, useRef, useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { Bot, Download, SlidersHorizontal } from 'lucide-react'
import {
  Avatar,
  ExportDialog,
  LanguageSwitcher,
  LayoutEditor,
  ProfileDialog,
  ProjectAgentsDialog,
  ThemeSwitcher,
  agentDisplayName,
  idsFromPath,
  useChat,
  useI18n,
  useProfile,
} from '@agentzoo/web-ui'
import { shellLayout } from '../layout'
import styles from './header.module.css'

export function Header() {
  const {
    newSession,
    topicSessions,
    busySessions,
    conversationTitle,
    renameConversation,
    projectId,
  } = useChat()
  const { pathname } = useLocation()
  const topicId = idsFromPath(pathname)?.topicId ?? null
  const { user, agent } = useProfile()
  const { t } = useI18n()
  const [showProfile, setShowProfile] = useState(false)
  const [showLayout, setShowLayout] = useState(false)
  const [showExport, setShowExport] = useState(false)
  const [showProjectAgents, setShowProjectAgents] = useState(false)
  const nickname = user.nickname?.trim() || t('message.you')
  // The group-chat title, WeChat style: the project's name plus the member
  // count (the local user plus every agent with a session in the
  // conversation).
  const group = useMemo(() => {
    const agents = new Set(topicSessions.map((s) => s.agent))
    const count = agents.size + 1
    const names = Array.from(agents)
      .map((name) => agentDisplayName(name, agent(name), t) || name)
      .sort((a, b) => a.localeCompare(b))
    return { count, names }
  }, [topicSessions, agent, t])
  const running = topicSessions.some((s) => busySessions.has(s.session_id))
  // The group's name: the conversation's title (LLM-generated or renamed),
  // falling back to the static "group chat" label until one exists.
  const title = conversationTitle.trim() || t('member.defaultTitle')
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  useEffect(() => {
    if (editing) inputRef.current?.focus()
  }, [editing])
  const startEdit = () => {
    setDraft(conversationTitle)
    setEditing(true)
  }
  const commit = async () => {
    setEditing(false)
    const next = draft.trim()
    if (!next || next === conversationTitle) return
    if (topicId) await renameConversation(topicId, next)
  }
  return (
    <header className={styles.header}>
      <div className={styles.titleWrap} title={group.names.join('、')}>
        {editing ? (
          <input
            ref={inputRef}
            className={styles.titleInput}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => void commit()}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void commit()
              if (e.key === 'Escape') setEditing(false)
            }}
            aria-label={t('member.renameTitle')}
          />
        ) : (
          <button
            type="button"
            className={styles.groupTitleBtn}
            onClick={startEdit}
            title={t('member.renameTitle')}
          >
            <span className={styles.groupTitle}>{title}</span>
          </button>
        )}
        <span className={styles.groupCount}>
          {t('member.count', { count: String(group.count) })}
        </span>
        {running && (
          <span
            className={styles.liveDot}
            title={t('sidebar.status.busy')}
            aria-hidden
          />
        )}
      </div>
      <div className={styles.actions}>
        <Link className={styles.link} to="/">
          {t('header.home')}
        </Link>
        <LanguageSwitcher />
        <ThemeSwitcher />
        {projectId && (
          <button
            type="button"
            className={styles.btn}
            onClick={() => setShowProjectAgents(true)}
            title={t('header.projectAgents')}
            aria-label={t('header.projectAgents')}
          >
            <Bot size={14} strokeWidth={1.8} aria-hidden />
          </button>
        )}
        <button
          type="button"
          className={styles.btn}
          onClick={() => setShowLayout(true)}
          title={t('layout.customize')}
          aria-label={t('layout.customize')}
        >
          <SlidersHorizontal size={14} strokeWidth={1.8} aria-hidden />
        </button>
        <button
          type="button"
          className={styles.btn}
          onClick={() => setShowExport(true)}
          title={t('header.export')}
          aria-label={t('header.export')}
        >
          <Download size={14} strokeWidth={1.8} aria-hidden />
        </button>
        <button className={styles.btnPrimary} onClick={() => void newSession()}>
          {t('header.newChat')}
        </button>
        <button
          type="button"
          className={styles.userBtn}
          onClick={() => setShowProfile(true)}
          title={t('header.profile')}
        >
          <Avatar src={user.avatar} name={nickname} size={20} />
          <span className={styles.userName}>{nickname}</span>
        </button>
      </div>
      {showProfile && <ProfileDialog onClose={() => setShowProfile(false)} />}
      {showProjectAgents && projectId && (
        <ProjectAgentsDialog
          projectId={projectId}
          onClose={() => setShowProjectAgents(false)}
        />
      )}
      {showLayout && (
        <LayoutEditor
          layoutId="shell"
          spec={shellLayout}
          onClose={() => setShowLayout(false)}
        />
      )}
      {showExport && <ExportDialog onClose={() => setShowExport(false)} />}
    </header>
  )
}
