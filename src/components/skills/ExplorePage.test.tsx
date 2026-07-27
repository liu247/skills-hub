import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import ExplorePage from './ExplorePage'

const t = ((key: string) => ({
  addSkills: 'Add Skills',
  'exploreTabs.online': 'Online search',
  'exploreTabs.git': 'Git repository',
  'exploreTabs.local': 'Local directory',
  exploreFilterPlaceholder: 'Search skills',
  manualAdd: 'Manual add',
  'aiParse.open': 'AI parse',
  exploreSourceHint: 'Source hint',
  exploreEmpty: 'No skills found',
  searchEmpty: 'No search results',
} as Record<string, string>)[key] ?? key) as unknown as TFunction

describe('ExplorePage', () => {
  it('provides a direct AI parsing entry point beside manual Skill add', () => {
    const markup = renderToStaticMarkup(
      <ExplorePage
        featuredSkills={[]}
        featuredLoading={false}
        exploreFilter=""
        searchResults={[]}
        searchLoading={false}
        managedSkills={[]}
        loading={false}
        onExploreFilterChange={() => undefined}
        onInstallSkill={() => undefined}
        onOpenManualAdd={() => undefined}
        onOpenAiParse={() => undefined}
        t={t}
      />,
    )

    expect(markup).toContain('AI parse')
    expect(markup).toContain('Manual add')
    expect(markup).toMatch(/AI parse[\s\S]*Manual add/)
  })
})
