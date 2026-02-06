ALTER TABLE funding_rate
ADD COLUMN ts_hour TIMESTAMP
GENERATED ALWAYS AS (DATE_TRUNC('hour', timestamp)) STORED;

ALTER TABLE funding_rate
ADD COLUMN ts_day TIMESTAMP
GENERATED ALWAYS AS (DATE_TRUNC('day', timestamp)) STORED;

ALTER TABLE funding_rate
ADD COLUMN ts_week TIMESTAMP
GENERATED ALWAYS AS (DATE_TRUNC('week', timestamp)) STORED;

ALTER TABLE funding_rate
ADD COLUMN ts_month TIMESTAMP
GENERATED ALWAYS AS (DATE_TRUNC('month', timestamp)) STORED;

CREATE INDEX idx_fr_hour
ON funding_rate (symbol, ts_hour);

CREATE INDEX idx_fr_day
ON funding_rate (symbol, ts_day);

CREATE INDEX idx_fr_week
ON funding_rate (symbol, ts_week);

CREATE INDEX idx_fr_month
ON funding_rate (symbol, ts_month);;
