-- Recurring tasks: store a 5-field POSIX cron schedule on the task row.
-- Completing a recurring task materializes the next occurrence and nulls
-- this column on the completed row (history).

ALTER TABLE tasks ADD COLUMN repeat_cron TEXT;
