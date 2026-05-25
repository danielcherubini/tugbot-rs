INSERT INTO features (name, enabled) VALUES ('is_this_real', true)
ON CONFLICT (name) DO NOTHING;
