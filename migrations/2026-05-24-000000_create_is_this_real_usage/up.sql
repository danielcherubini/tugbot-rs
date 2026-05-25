CREATE TABLE "is_this_real_usage" (
  "id" SERIAL PRIMARY KEY,
  "user_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "last_used_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "created_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, guild_id)
);
