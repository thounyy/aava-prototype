-- Sessions table (off-chain only)
-- Note: Viewers, streams and subscriptions live on-chain (Sui blockchain)
-- This table only stores session data for high-throughput operations

CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(255) NOT NULL,
    stream_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'created', -- created, active, completed, error
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_stream_id ON sessions(stream_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);





