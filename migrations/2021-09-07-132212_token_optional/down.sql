UPDATE shares SET token = '' WHERE token IS NULL;
ALTER TABLE shares ALTER COLUMN token SET NOT NULL;
