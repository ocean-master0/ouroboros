-- migrations/001_init.sql
-- Project Ouroboros — initial schema
-- All three tables created from Day 1 so the schema doesn't need a
-- breaking change for full release (MVP_TECH_DOC.md §7).

CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    email       VARCHAR(255) UNIQUE NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS request_logs (
    id          BIGSERIAL PRIMARY KEY,
    endpoint    VARCHAR(100) NOT NULL,
    status_code SMALLINT NOT NULL,
    latency_us  INTEGER NOT NULL,
    logged_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS bench_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mode            VARCHAR(20) NOT NULL,
    concurrency     INTEGER NOT NULL,
    duration_secs   INTEGER NOT NULL,
    peak_rps        BIGINT,
    avg_rps         BIGINT,
    p99_ms          FLOAT,
    error_rate_pct  FLOAT,
    started_at      TIMESTAMPTZ NOT NULL,
    ended_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_request_logs_logged_at ON request_logs (logged_at);
CREATE INDEX IF NOT EXISTS idx_bench_sessions_started_at ON bench_sessions (started_at);
