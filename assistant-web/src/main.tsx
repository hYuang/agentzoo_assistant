import React, { useEffect, useState } from 'react'
import ReactDOM from 'react-dom/client'
import { MemoryRouter } from 'react-router-dom'
import {
  ChatProvider,
  I18nProvider,
  ProfileProvider,
  SettingsProvider,
  ThemeProvider,
  listPlugins,
  type PluginInfo,
} from '@agentzoo/web-ui'
import '@agentzoo/web-ui/global.css'
import 'katex/dist/katex.min.css'
import { AssistantDashboard } from './assistant/AssistantDashboard'

function projectIdFromUrl(): string {
  return new URLSearchParams(window.location.search).get('projectId') ?? ''
}

function App() {
  const projectId = projectIdFromUrl()
  const [plugin, setPlugin] = useState<PluginInfo | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listPlugins()
      .then((res) => {
        const found = res.plugins.find((item) => item.id === 'assistant')
        if (!found) throw new Error('assistant plugin is not available')
        setPlugin(found)
      })
      .catch((err: unknown) => setError(String(err)))
  }, [])

  if (!projectId) return <p>Missing projectId</p>
  if (error) return <p>{error}</p>
  if (!plugin) return <p>Loading assistant…</p>

  return (
    <I18nProvider>
      <ThemeProvider>
        <SettingsProvider>
          <ProfileProvider>
            <MemoryRouter initialEntries={[`/project/${projectId}`]}>
              <ChatProvider>
                <AssistantDashboard
                  plugin={plugin}
                  projectId={projectId}
                  onProjectChange={() => undefined}
                />
              </ChatProvider>
            </MemoryRouter>
          </ProfileProvider>
        </SettingsProvider>
      </ThemeProvider>
    </I18nProvider>
  )
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
