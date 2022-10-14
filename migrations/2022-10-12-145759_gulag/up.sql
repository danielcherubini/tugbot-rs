CREATE TABLE "servers" (
  "id" SERIAL PRIMARY KEY,
  "guild_id" bigint NOT NULL,
  "gulag_id" bigint NOT NULL,
  "gulag_members" int UNIQUE
);
--
-- CREATE TABLE "gulag_members" (
--   "id" SERIAL PRIMARY KEY,
--   "user_id" bigint NOT NULL,
--   "created_at" varchar NOT NULL
-- );
--
-- ALTER TABLE "gulag_members" ADD FOREIGN KEY ("id") REFERENCES "servers" ("gulag_members");
