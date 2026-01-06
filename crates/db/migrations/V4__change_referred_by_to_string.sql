-- Drop the old foreign key constraint
ALTER TABLE users DROP CONSTRAINT IF EXISTS fk_referred_by;

-- Change referred_by column from UUID to VARCHAR to store referral codes
ALTER TABLE users ALTER COLUMN referred_by TYPE VARCHAR(50);

-- Add new foreign key constraint on referral_code
-- This ensures referred_by must reference an existing user's referral_code
ALTER TABLE users 
    ADD CONSTRAINT fk_referred_by_referral_code
    FOREIGN KEY (referred_by)
    REFERENCES users(referral_code)
    ON DELETE SET NULL;

