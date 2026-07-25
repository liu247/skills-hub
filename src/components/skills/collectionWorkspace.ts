import type { ManagedSkill } from './types'

export type CollectionView = 'series' | 'skills'

export const shouldShowCollectionsLanding = (
  collectionView: CollectionView,
  searchQuery: string,
) => collectionView === 'series' && !searchQuery.trim()

export const filterSkillsForCollection = (
  skills: ManagedSkill[],
  collectionView: CollectionView,
  activeCollection: string | null,
  searchQuery: string,
) => {
  if (collectionView !== 'skills' || searchQuery.trim()) return skills
  return skills.filter((skill) => {
    if (activeCollection === null) return !skill.collection?.trim()
    return skill.collection === activeCollection
  })
}
