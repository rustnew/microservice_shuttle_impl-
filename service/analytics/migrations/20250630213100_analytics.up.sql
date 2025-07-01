-- Add up migration script here
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR NOT NULL,
    post_id INTEGER,
    data JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);