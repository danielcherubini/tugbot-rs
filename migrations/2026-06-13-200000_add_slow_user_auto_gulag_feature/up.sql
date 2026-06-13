-- Add the slow_user_auto_gulag feature flag (disabled by default).
-- When enabled, users in SLOW_USER_IDS get auto-gulagged on ANY bot mention
-- in #ask-tugbot. When disabled (default), slow users only get the longer 2h
-- cooldown. See src/handlers/mention.rs.
INSERT INTO features (name, enabled) VALUES ('slow_user_auto_gulag', false)
ON CONFLICT (name) DO NOTHING;
