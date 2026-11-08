import { registerPanel } from '@agentzoo/web-ui'
import { ProjectSidebar } from './ProjectSidebar/ProjectSidebar'
import { Header } from './Header/Header'

// Host panels for the conversation shell. The reusable chat panels (members,
// transcript, composer, toolbox) are registered by ./panels.ts.
registerPanel('sidebar', ProjectSidebar)
registerPanel('header', Header)
