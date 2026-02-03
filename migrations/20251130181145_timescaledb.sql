CREATE TABLE line (
  time        TIMESTAMPTZ       NOT NULL,
  nb_people   INT               NOT NULL,
  source      VARCHAR(5)	NULL
)
WITH (
  timescaledb.hypertable,
  timescaledb.partition_column='time'
);
