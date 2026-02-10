CREATE TABLE "ai_slop_usage" (
  "id" SERIAL PRIMARY KEY,
  "user_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "usage_count" int NOT NULL DEFAULT 0,
  "last_slop_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "created_at" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, guild_id)
);
