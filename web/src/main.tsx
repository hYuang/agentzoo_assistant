import { mountApp } from '@agentzoo/web-ui'
import { AssistantHome } from './home'
import { AssistantShell } from './AssistantShell'

mountApp({ home: <AssistantHome />, shell: <AssistantShell /> })
