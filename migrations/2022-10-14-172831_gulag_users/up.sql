CREATE TABLE "gulag_users" (
  "id" SERIAL PRIMARY KEY,
  "user_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "gulag_role_id" bigint NOT NULL,
  "channel_id" bigint NOT NULL,
  "in_gulag" boolean NOT NULL,
  "gulag_length" int NOT NULL,
  "created_at" timestamp NOT NULL
);
