SELECT
  hash,
  authored_timestamp
FROM
  DHtOp
WHERE
  DhtOp.authored_timestamp >= :from
  AND DhtOp.authored_timestamp < :to
ORDER BY
  authored_timestamp ASC
LIMIT
  :limit
