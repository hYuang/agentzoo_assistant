import { mountApp } from '@agentzoo/web-ui'
import './panels'
import './shell/panels'
import { AssistantHome } from './home'
import { AssistantShell } from './AssistantShell'

mountApp({ home: <AssistantHome />, shell: <AssistantShell /> })
