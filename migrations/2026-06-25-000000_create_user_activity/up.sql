CREATE TABLE user_activity (
    user_id         BIGINT      NOT NULL,
    guild_id        BIGINT      NOT NULL,
    last_message_at TIMESTAMP   NOT NULL,
    created_at      TIMESTAMP   NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, guild_id)
);

CREATE INDEX idx_user_activity_guild_last_message
    ON user_activity (guild_id, last_message_at);
