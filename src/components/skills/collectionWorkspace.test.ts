import { describe, expect, it } from 'vitest'
import {
  filterSkillsForCollection,
  shouldShowCollectionsLanding,
} from './collectionWorkspace'
import type { ManagedSkill } from './types'

const skill = (id: string, collection: string | null): ManagedSkill => ({
  id,
  name: id,
  source_type: 'git',
  central_path: `/skills/${id}`,
  created_at: 0,
  updated_at: 0,
  enabled: true,
  status: 'ok',
  tags: [],
  targets: [],
  collection,
})

describe('collection workspace', () => {
  it('shows the collection landing until a search or collection is selected', () => {
    expect(shouldShowCollectionsLanding('series', '')).toBe(true)
    expect(shouldShowCollectionsLanding('series', 'pdf')).toBe(false)
    expect(shouldShowCollectionsLanding('skills', '')).toBe(false)
  })

  it('shows only the selected collection while retaining uncategorized skills separately', () => {
    const skills = [skill('paper', 'Research'), skill('chart', 'Research'), skill('misc', null)]

    expect(filterSkillsForCollection(skills, 'skills', 'Research', '')).toEqual([
      skills[0],
      skills[1],
    ])
    expect(filterSkillsForCollection(skills, 'skills', null, '')).toEqual([skills[2]])
    expect(filterSkillsForCollection(skills, 'skills', 'Research', 'paper')).toEqual(skills)
  })
})
