ALTER TABLE workspace_observation_events ADD COLUMN observation_id TEXT;
UPDATE workspace_observation_events
SET observation_id = 'file-observation:migrated:' || lower(hex(randomblob(16)))
WHERE observation_id IS NULL;
CREATE UNIQUE INDEX workspace_observation_events_identity
ON workspace_observation_events(observation_id);

ALTER TABLE language_observations ADD COLUMN observation_id TEXT;
UPDATE language_observations
SET observation_id = 'language-observation:migrated:' || lower(hex(randomblob(16)))
WHERE observation_id IS NULL;
CREATE UNIQUE INDEX language_observations_identity
ON language_observations(observation_id);
