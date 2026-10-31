import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import {
  getSynapseStatus,
  loginStudio,
  logoutStudio,
  type SynapseLoginRequest,
  type SynapseSession,
  type SynapseStatus,
} from './api'

interface StudioSessionValue {
  status: SynapseStatus | null
  session: SynapseSession | null
  canPublish: boolean
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
  login: (request: SynapseLoginRequest) => Promise<void>
  logout: () => Promise<void>
}

const StudioSessionContext = createContext<StudioSessionValue | null>(null)

interface StudioSessionProviderProps {
  projectId: string
  pluginId: string
  runId: string
  children: ReactNode
}

export function StudioSessionProvider({
  projectId,
  pluginId,
  runId,
  children,
}: StudioSessionProviderProps) {
  const [status, setStatus] = useState<SynapseStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setStatus(await getSynapseStatus(projectId, pluginId, runId))
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectId, pluginId, runId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const login = useCallback(
    async (request: SynapseLoginRequest) => {
      setError(null)
      const result = await loginStudio(projectId, pluginId, runId, request)
      setStatus((current) =>
        current
          ? { ...current, session: result.session, can_publish: true }
          : current,
      )
    },
    [projectId, pluginId, runId],
  )

  const logout = useCallback(async () => {
    setError(null)
    await logoutStudio(projectId, pluginId, runId)
    setStatus((current) =>
      current
        ? {
            ...current,
            session: null,
            can_publish: current.config.configured,
          }
        : current,
    )
  }, [projectId, pluginId, runId])

  const value = useMemo<StudioSessionValue>(
    () => ({
      status,
      session: status?.session ?? null,
      canPublish: status?.can_publish ?? false,
      loading,
      error,
      refresh,
      login,
      logout,
    }),
    [status, loading, error, refresh, login, logout],
  )

  return (
    <StudioSessionContext.Provider value={value}>
      {children}
    </StudioSessionContext.Provider>
  )
}

export function useStudioSession(): StudioSessionValue {
  const value = useContext(StudioSessionContext)
  if (!value) {
    throw new Error('useStudioSession must be used inside StudioSessionProvider')
  }
  return value
}
