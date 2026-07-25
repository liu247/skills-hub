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
    'mcpImport.title': 'Add MCP',
    'manageTabs.updates': 'Updates',
    settings: 'Settings',
    'sidebar.collapse': 'Collapse sidebar',
    'sidebar.expand': 'Expand sidebar',
  }
  return translations[key] ?? key
}) as unknown as TFunction

describe('Header', () => {
  it('places MCP navigation alongside the workspace actions', () => {
    const markup = renderToStaticMarkup(
      <Header
        activeView="mcp"
        managementTab="tags"
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

    expect(markup).toMatch(
      /<nav class="sidebar-nav" aria-label="Workspace">[\s\S]*?<span>My Skills<\/span>[\s\S]*?<span>Add Skills<\/span>[\s\S]*?<span>MCP<\/span>[\s\S]*?<span>Add MCP<\/span>[\s\S]*?<\/nav>/,
    )
  })
})
