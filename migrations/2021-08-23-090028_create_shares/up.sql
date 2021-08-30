CREATE TABLE shares (
    name VARCHAR(255) PRIMARY KEY,  -- Name part of the link, also used as a local file name.
    expiry TIMESTAMP,               -- When the share will be deleted.
    token VARCHAR(255) NOT NULL,    -- Used for managing the share.
    kind SMALLINT NOT NULL,         -- 1 = link, 2 = paste, 3 = file
    link VARCHAR(2047),             -- Redirect URL for shortlinks.
    language VARCHAR(31),           -- Syntax highlighting hint for pastes.
    mime_type VARCHAR(127)          -- MIME type for files.
)
