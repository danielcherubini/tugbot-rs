ALTER TABLE "message_votes"
  RENAME COLUMN current_vote_tally TO vote_tally;

ALTER TABLE "message_votes"
  DROP COLUMN total_vote_tally;
