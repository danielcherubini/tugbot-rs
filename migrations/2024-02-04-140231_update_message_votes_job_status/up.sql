CREATE TYPE "job_status" AS ENUM (
  'created',
  'running',
  'done',
  'failure'
);

ALTER TABLE "message_votes"
  ADD COLUMN "job_status" job_status NOT NULL;
