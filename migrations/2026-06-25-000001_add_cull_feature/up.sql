-- Add the cull feature flag (disabled by default).
-- When enabled, admins can use /cull to kick inactive members.
INSERT INTO features (name, enabled) VALUES ('cull', false)
ON CONFLICT (name) DO NOTHING;
