-- Drop the existing case-sensitive unique index on email
DROP INDEX IF EXISTS users_email_key;

-- Create a case-insensitive unique index on email
CREATE UNIQUE INDEX users_email_unique_ci ON users (LOWER(email));
