#!/bin/sh
set -eu

# Helm values should pass this as env; default true.
FAIL_RELEASE_ON_ERROR="${FAIL_RELEASE_ON_ERROR:-true}"

echo "[logto-bootstrap] starting"
echo "[logto-bootstrap] FAIL_RELEASE_ON_ERROR=${FAIL_RELEASE_ON_ERROR}"

# Trust mkcert root (harmless if none)
if command -v update-ca-certificates >/dev/null 2>&1; then
  update-ca-certificates || true
fi

# Require DB_URL for kubernetes-native setup (passed by Helm job)
if [ -z "${DB_URL:-}" ]; then
  echo "[logto-bootstrap] ERROR: DB_URL is required"
  exit 1
fi

# Wait for Postgres readiness (no pg_isready required)
echo "[logto-bootstrap] waiting for postgres..."
MAX_ATTEMPTS="${DB_WAIT_MAX_ATTEMPTS:-60}"
SLEEP_SECS="${DB_WAIT_SLEEP_SECONDS:-2}"

i=1
while [ "$i" -le "$MAX_ATTEMPTS" ]; do
  if node - <<'NODE'
const { createPool, sql } = require('@silverhand/slonik');
(async () => {
  const pool = await createPool(process.env.DB_URL);
  try {
    await pool.query(sql`select 1 as ok`);
    process.exit(0);
  } catch (e) {
    process.exit(1);
  } finally {
    await pool.end();
  }
})();
NODE
  then
    echo "[logto-bootstrap] postgres is ready"
    break
  fi

  echo "[logto-bootstrap] postgres not ready (attempt $i/$MAX_ATTEMPTS); sleeping ${SLEEP_SECS}s"
  i=$((i + 1))
  sleep "$SLEEP_SECS"
done

if [ "$i" -gt "$MAX_ATTEMPTS" ]; then
  echo "[logto-bootstrap] ERROR: postgres did not become ready"
  exit 1
fi

run_or_handle_error() {
  echo "[logto-bootstrap] $*"
  if "$@"; then
    return 0
  fi

  echo "[logto-bootstrap] ERROR running: $*"
  if [ "$FAIL_RELEASE_ON_ERROR" = "false" ]; then
    echo "[logto-bootstrap] continuing because FAIL_RELEASE_ON_ERROR=false"
    return 0
  fi
  return 1
}

# Deploy database alterations (schema migrations)
run_or_handle_error npm run alteration deploy

# Seed (idempotent enough for Logto; still safe to run multiple times)
run_or_handle_error npm run cli db seed -- --swe

# Custom setup (idempotent UPSERTs)
# Ensure path matches what your ConfigMap mounts in the Job.
run_or_handle_error node /custom-setup/index.js

echo "[logto-bootstrap] done"
