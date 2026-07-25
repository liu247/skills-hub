import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import Header from './Header'

const t = ((key: string) => {
  const translations: Record<string, string> = {
    appName: 'Skills Hub',
    workspaceSubtitle: 'AI agent workspace',
    workspace: 'Workspace',
    navMySkills: 'My Skills',
    addSkills: 'Add Skills',
    navManageCenter: 'Management Center',
    'manageTabs.tags': 'Tags',
    'manageTabs.tools': 'Tools',
    'manageTabs.mcp': 'MCP',
    'manageTabs.updates': 'Updates',
    settings: 'Settings',
    'sidebar.collapse': 'Collapse sidebar',
    'sidebar.expand': 'Expand sidebar',
  }
  return translations[key] ?? key
}) as unknown as TFunction

describe('Header', () => {
  it('exposes MCP in the management navigation', () => {
    const markup = renderToStaticMarkup(
      <Header
        activeView="manage"
        managementTab="mcp"
        skillCount={0}
        tagCount={0}
        toolCount={0}
        updateCount={0}
        appVersion="0.8.1"
        updateAvailableVersion={null}
        updateChecking={false}
        updateInstalling={false}
        updateDone={false}
        collapsed={false}
        onToggleCollapsed={() => undefined}
        onOpenSettings={() => undefined}
        onOpenUpdate={() => undefined}
        onViewChange={() => undefined}
        onManagementTabChange={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('>MCP</span>')
  })
})
