CREATE TABLE "message_votes" (
  "message_id" bigint PRIMARY KEY,
  "channel_id" bigint NOT NULL,
  "guild_id" bigint NOT NULL,
  "user_id" bigint NOT NULL,
  "vote_tally" int NOT NULL,
  "voters" bigint[] NOT NULL
);
