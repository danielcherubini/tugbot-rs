ALTER TABLE "message_votes"
  RENAME COLUMN vote_tally TO current_vote_tally;

ALTER TABLE "message_votes"
  ADD COLUMN total_vote_tally int NOT NULL DEFAULT 0;
