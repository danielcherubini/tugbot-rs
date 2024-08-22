ALTER TABLE "message_votes"
  RENAME COLUMN vote_tally TO total_vote_tally;

ALTER TABLE "message_votes"
  ADD COLUMN current_vote_tally int NOT NULL DEFAULT 0;

-- UPDATE "message_votes"
  -- SET total_vote_tally = current_vote_tally, current_vote_tally = 0
  -- WHERE current_vote_tally > 0 AND (current_vote_tally % 5) = 0;
