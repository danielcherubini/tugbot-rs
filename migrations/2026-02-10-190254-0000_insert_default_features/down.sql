-- Remove default features
DELETE FROM features WHERE name IN (
    'twitter',
    'tiktok',
    'instagram',
    'bsky',
    'ai_slop',
    'teh'
);
