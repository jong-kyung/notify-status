#!/usr/bin/env bash
set -euo pipefail

version="${1:?Pass the published version}"

# Retry the entire install because the root and platform packages can reach
# registry replicas at different times.
attempt=1
until [ "$attempt" -gt 5 ]; do
  install_dir="$(mktemp -d)"
  if (
    set -euo pipefail
    cd "$install_dir"
    npm init -y >/dev/null
    npm install "notify-status@$version"

    # The root package must load the platform's optional dependency.
    if compgen -G 'node_modules/notify-status/*.node' > /dev/null; then
      echo "::error::The root package must not bundle native addons"
      exit 1
    fi

    cat > check.cjs <<'EOF'
const assert = require('node:assert')
const m = require('notify-status')
m.getNotificationStatus().then((r) => {
  console.log(JSON.stringify(r))
  assert.strictEqual(typeof r.authorization, 'string')
  if (process.platform === 'linux') {
    assert.deepStrictEqual(r, { authorization: 'unsupported', doNotDisturb: false, platform: 'linux', reason: 'unsupportedPlatform' })
  }
})
EOF
    cat > check.mjs <<'EOF'
import assert from 'node:assert'
import { getNotificationStatus } from 'notify-status'
const r = await getNotificationStatus()
console.log(JSON.stringify(r))
assert.strictEqual(typeof r.authorization, 'string')
if (process.platform === 'linux') {
  assert.deepStrictEqual(r, { authorization: 'unsupported', doNotDisturb: false, platform: 'linux', reason: 'unsupportedPlatform' })
}
EOF

    node check.cjs && node check.mjs
  ); then
    echo "Installed and exercised notify-status@$version on attempt $attempt"
    exit 0
  fi
  echo "Attempt $attempt failed; the registry may still be propagating"
  attempt=$((attempt + 1))
  sleep 20
done
echo "::error::notify-status@$version did not install and load after 5 attempts"
exit 1
