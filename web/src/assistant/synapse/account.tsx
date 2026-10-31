import { useState, type FormEvent } from 'react'
import { LogIn, LogOut, UserRound } from 'lucide-react'
import { useStudioSession } from './session'
import { useAssistantI18n } from '../i18n'
import styles from '../assistant.module.css'

export function StudioAccountPanel() {
  const { t } = useAssistantI18n()
  const { status, session, loading, error, login, logout } = useStudioSession()
  const [open, setOpen] = useState(false)
  const [tenant, setTenant] = useState('')
  const [account, setAccount] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [loginError, setLoginError] = useState<string | null>(null)

  const defaultTenant = status?.config.tenant_id ?? ''
  const effectiveTenant = tenant || defaultTenant
  const displayName = session?.name || session?.user_account || ''
  const initial = displayName.trim().charAt(0).toUpperCase() || 'S'

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (busy) return
    setBusy(true)
    setLoginError(null)
    try {
      await login({
        tenant_id: effectiveTenant.trim(),
        user_account: account.trim(),
        password,
      })
      setOpen(false)
      setPassword('')
    } catch (e) {
      setLoginError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={`${styles.railSection} ${styles.railFooter}`}>
      <div className={styles.studioAccountHead}>
        <span className={styles.studioAccountBadge}>
          <UserRound size={12} strokeWidth={2} aria-hidden />
        </span>
        {t('studioAccount')}
      </div>

      {session ? (
        <div className={styles.studioAccountCard}>
          <span className={styles.studioAccountAvatar}>{initial}</span>
          <div className={styles.studioAccountIdentity}>
            <strong>{displayName}</strong>
            <span>{session.tenant_id}</span>
          </div>
          <button
            type="button"
            className={styles.studioAccountIconButton}
            title={t('synapseLogout')}
            aria-label={t('synapseLogout')}
            onClick={() => void logout()}
          >
            <LogOut size={14} strokeWidth={1.8} aria-hidden />
          </button>
        </div>
      ) : open ? (
        <form className={styles.studioAccountForm} onSubmit={(e) => void submit(e)}>
          <label className={styles.studioAccountField}>
            <span>{t('synapseTenant')}</span>
            <input
              value={effectiveTenant}
              autoComplete="off"
              onChange={(e) => setTenant(e.target.value)}
            />
          </label>
          <label className={styles.studioAccountField}>
            <span>{t('synapseAccount')}</span>
            <input
              value={account}
              autoComplete="username"
              onChange={(e) => setAccount(e.target.value)}
            />
          </label>
          <label className={styles.studioAccountField}>
            <span>{t('synapsePassword')}</span>
            <input
              type="password"
              value={password}
              autoComplete="current-password"
              onChange={(e) => setPassword(e.target.value)}
            />
          </label>
          <div className={styles.studioAccountActions}>
            <button
              type="button"
              className={styles.studioAccountSecondary}
              disabled={busy}
              onClick={() => setOpen(false)}
            >
              {t('synapseCancel')}
            </button>
            <button
              type="submit"
              className={styles.studioAccountPrimary}
              disabled={busy || !account.trim() || !password}
            >
              <LogIn size={13} strokeWidth={2} aria-hidden />
              {busy ? t('synapseLoggingIn') : t('synapseLogin')}
            </button>
          </div>
          {(loginError || error) && (
            <div className={styles.studioAccountError}>{loginError || error}</div>
          )}
        </form>
      ) : (
        <>
          <button
            type="button"
            className={styles.studioAccountButton}
            onClick={() => setOpen(true)}
          >
            <LogIn size={14} strokeWidth={2} aria-hidden />
            {t('synapseLogin')}
          </button>
          {error && <div className={styles.studioAccountError}>{error}</div>}
        </>
      )}

      {loading && <div className={styles.studioAccountHint}>{t('synapseLoading')}</div>}
    </div>
  )
}
