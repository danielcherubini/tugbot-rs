INSERT INTO features (name, enabled) VALUES ('goku_poll', true)
ON CONFLICT (name) DO NOTHING;
