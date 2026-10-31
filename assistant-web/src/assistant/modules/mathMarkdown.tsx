import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import rehypeKatex from 'rehype-katex'
import type { Components } from 'react-markdown'
import { normalizeMath } from '@agentzoo/web-ui'

// Inline mode unwraps the paragraph renderer, so a flow of text/math can sit
// inside a table cell, option label, or flashcard without block layout.
const inlineComponents: Components = {
  p: ({ children }) => <>{children}</>,
}

export function MathMarkdown({
  text,
  inline = false,
  className,
}: {
  text: string
  inline?: boolean
  className?: string
}) {
  const content = (
    <Markdown
      remarkPlugins={[remarkGfm, remarkMath]}
      rehypePlugins={[rehypeKatex]}
      components={inline ? inlineComponents : undefined}
    >
      {normalizeMath(text)}
    </Markdown>
  )
  return inline ? (
    <span className={className}>{content}</span>
  ) : (
    <div className={className}>{content}</div>
  )
}
