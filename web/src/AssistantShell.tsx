import { Layout, useLayoutSpec } from '@agentzoo/web-ui'
import { shellLayout } from './shell/layout'
import styles from './shell/shell.module.css'

export function AssistantShell() {
  const spec = useLayoutSpec('shell', shellLayout)
  return <Layout id="shell" spec={spec} className={styles.shell} />
}
