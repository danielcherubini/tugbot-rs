ALTER TABLE "message_votes"
  RENAME COLUMN total_vote_tally TO vote_tally;

ALTER TABLE "message_votes"
  DROP COLUMN current_vote_tally;
