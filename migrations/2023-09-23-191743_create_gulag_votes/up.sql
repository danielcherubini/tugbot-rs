CREATE TABLE "gulag_votes" (
  "id" SERIAL PRIMARY KEY,
  "requester_id" bigint NOT NULL,
  "sender_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "channel_id" bigint NOT NULL,
  "gulag_role_id" bigint NOT NULL,
  "processed" boolean NOT NULL,
  "message_id" bigint NOT NULL,
  "created_at" timestamp NOT NULL
);

CREATE TABLE "reversal_of_fortunes" (
  "user_id" bigint PRIMARY KEY,
  "current_percentage" bigint NOT NULL
);

