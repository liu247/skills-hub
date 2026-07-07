UPDATE skills SET collection = 'superpowers' WHERE name IN (
  'brainstorming','dispatching-parallel-agents','executing-plans','finishing-a-development-branch',
  'receiving-code-review','requesting-code-review','subagent-driven-development','systematic-debugging',
  'test-driven-development','using-git-worktrees','using-superpowers','verification-before-completion',
  'writing-plans','writing-skills'
);
UPDATE skills SET collection = 'PaperSpine' WHERE name IN (
  'paper-spine','paper-spine-audit','paper-spine-build','paper-spine-citation','paper-spine-humanize',
  'paper-spine-intake','paper-spine-latex','paper-spine-research','paper-spine-rewrite','paper-spine-translate',
  'paper-spine-ui','paper-spine-update'
);
UPDATE skills SET collection = 'nature-skills' WHERE name IN (
  'nature-academic-search','nature-citation','nature-data','nature-figure','nature-paper2ppt',
  'nature-polishing','nature-reader','nature-response','nature-writing'
);
UPDATE skills SET collection = 'claude-scientific-writer' WHERE name IN (
  'citation-management','hypothesis-generation','latex-posters','literature-review','peer-review',
  'research-grants','scientific-schematics','scientific-slides','scientific-writer','scientific-writing',
  'venue-templates'
);
UPDATE skills SET collection = 'bggg-skills' WHERE name IN (
  'bggg-creator-image2ppt','bggg-creator-image2psd','bggg-skill-taotie'
);
UPDATE skills SET collection = 'openai-skills' WHERE name IN (
  'hatch-pet'
);
UPDATE skills SET collection = 'codex-utils' WHERE name IN (
  'delete-session','remote-reconnection','remote-update'
);
