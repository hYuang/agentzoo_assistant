import { registerPanel } from '@agentzoo/web-ui'
import { MemberBar } from '@agentzoo/web-ui/components/chat/MemberBar/MemberBar.tsx'
import { MessageList } from '@agentzoo/web-ui/components/chat/MessageList/MessageList.tsx'
import { InputBar } from '@agentzoo/web-ui/components/chat/InputBar/InputBar.tsx'
import { Toolbox } from '@agentzoo/web-ui/components/dev/Toolbox/Toolbox.tsx'

registerPanel('members', MemberBar)
registerPanel('transcript', MessageList)
registerPanel('composer', InputBar)
registerPanel('toolbox', Toolbox)
