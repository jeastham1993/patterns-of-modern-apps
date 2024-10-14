-- Add migration script here
CREATE TABLE loyalty_transaction (
  customer_id VARCHAR(255),
  date_epoch bigint,
  order_number VARCHAR(255),
  change REAL
);