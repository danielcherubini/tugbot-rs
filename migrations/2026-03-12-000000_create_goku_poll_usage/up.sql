CREATE TABLE goku_poll_usage (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 0,
    last_goku_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, guild_id)
);
