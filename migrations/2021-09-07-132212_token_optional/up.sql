-- When this migration is applied for the first time, the following UPDATE
-- statement should do nothing. It is only here because when the migration is
-- downgraded, NULL values have to be replaced with an empty string, and we
-- want to undo that when the migration is applied again.
UPDATE shares SET token = NULL WHERE token = '';

ALTER TABLE shares ALTER COLUMN token DROP NOT NULL;
