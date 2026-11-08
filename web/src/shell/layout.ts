import type { LayoutNode } from '@agentzoo/web-ui'

// The conversation page layout: sidebar | (header / transcript / composer) |
// toolbox. This is a plain data tree — reorder regions, flip `hidden`, adjust
// sizes, or swap a `panel` kind without touching the page component. Widths
// and collapse state are persisted per node id by the engine.
export const shellLayout: LayoutNode = {
  id: 'shell',
  dir: 'row',
  children: [
    {
      id: 'sidebar',
      panel: 'sidebar',
      resizable: 'left',
      collapsible: true,
      collapsedSize: 52,
      size: 240,
      min: 160,
      max: 480,
    },
    {
      id: 'center',
      dir: 'column',
      grow: true,
      children: [
        { id: 'header', panel: 'header', locked: true },
        { id: 'members', panel: 'members', locked: true },
        { id: 'transcript', panel: 'transcript', grow: true },
        { id: 'composer', panel: 'composer' },
      ],
    },
    {
      id: 'toolbox',
      panel: 'toolbox',
      resizable: 'right',
      collapsible: true,
      defaultCollapsed: true,
      collapsedSize: 48,
      size: 280,
      min: 200,
      max: 480,
    },
  ],
}
