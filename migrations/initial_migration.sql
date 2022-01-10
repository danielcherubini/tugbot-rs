CREATE TABLE "servers" (
  "id" SERIAL PRIMARY KEY,
  "server_name" varchar,
  "created_at" timestamp,
  "country_code" int,
  "tokens" Tokens,
  "config" Config
);

CREATE TABLE "Tokens" (
  "application_id" int PRIMARY KEY,
  "discord_token" varchar
);

CREATE TABLE "Config" (
  "guild_id" varchar PRIMARY KEY,
  "gulag_role_id" varchar,
  "gulag_channel_id" varchar
);

ALTER TABLE "Tokens" ADD FOREIGN KEY ("application_id") REFERENCES "servers" ("tokens");

ALTER TABLE "Config" ADD FOREIGN KEY ("guild_id") REFERENCES "servers" ("config");

