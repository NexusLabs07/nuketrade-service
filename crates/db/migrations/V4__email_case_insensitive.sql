-- Drop the existing unique constraint on email (this will also drop the associated index)
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_email_key;

-- Create a case-insensitive unique index on email
CREATE UNIQUE INDEX users_email_unique_ci ON users (LOWER(email));
