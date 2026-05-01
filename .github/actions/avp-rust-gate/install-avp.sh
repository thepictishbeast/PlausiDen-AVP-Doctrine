#!/usr/bin/env bash
# install-avp.sh — download the pinned `avp` binary from the doctrine
# repo's GitHub Release, verify its blake3 hash against the published
# manifest, then place it at $AVP_BIN.
#
# Required env:
#   AVP_VERSION — tag (e.g. "v0.1.0"). Must exist as a GH Release.
#   AVP_BIN     — absolute path to write the binary to.
#
# Optional env:
#   GITHUB_TOKEN — passed through for authenticated download (private repo).
#   AVP_TARGET   — release-asset target triple. Default: x86_64-unknown-linux-musl.
#
# SECURITY: blake3 verification is mandatory. The release pipeline publishes
# `avp-<target>.blake3` alongside the binary; if the local hash doesn't
# match, this script aborts with exit 70 (EX_PROTOCOL).

set -euo pipefail

if [[ -z "${AVP_VERSION:-}" ]]; then
    echo "::error::install-avp.sh: AVP_VERSION not set" >&2
    exit 64
fi
if [[ -z "${AVP_BIN:-}" ]]; then
    echo "::error::install-avp.sh: AVP_BIN not set" >&2
    exit 64
fi

target="${AVP_TARGET:-x86_64-unknown-linux-musl}"
asset_bin="avp-${target}"
asset_hash="${asset_bin}.blake3"

owner="thepictishbeast"
repo="PlausiDen-AVP-Doctrine"

# Fetch the release manifest via the API. Authenticate if a token is set
# (required for private repos like the doctrine repo today).
api_url="https://api.github.com/repos/${owner}/${repo}/releases/tags/${AVP_VERSION}"
echo "::group::resolve avp ${AVP_VERSION} from ${owner}/${repo}"
auth_headers=()
if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    auth_headers=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
fi
release_json=$(curl -fsSL "${auth_headers[@]}" \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "${api_url}")

# Pull the two asset URLs we need from the JSON (use python; jq isn't always
# present on the self-hosted runner image).
read -r bin_url hash_url < <(python3 - <<PY
import json, os, sys
data = json.loads(os.environ.get('REL_JSON') or sys.stdin.read())
bin_name = "${asset_bin}"
hash_name = "${asset_hash}"
b = h = ""
for a in data.get('assets', []):
    if a['name'] == bin_name:  b = a['url']
    if a['name'] == hash_name: h = a['url']
if not b or not h:
    print(f"::error::missing release assets {bin_name} or {hash_name}", file=sys.stderr)
    sys.exit(70)
print(b, h)
PY
REL_JSON="$release_json")
echo "::endgroup::"

# Download (binary + hash) using the API URLs (authenticated).
echo "::group::download avp + hash"
curl -fsSL "${auth_headers[@]}" \
    -H "Accept: application/octet-stream" \
    -o "${AVP_BIN}" "${bin_url}"
curl -fsSL "${auth_headers[@]}" \
    -H "Accept: application/octet-stream" \
    -o "${AVP_BIN}.blake3" "${hash_url}"
chmod +x "${AVP_BIN}"
echo "::endgroup::"

# Verify blake3.
echo "::group::verify blake3"
if command -v b3sum >/dev/null 2>&1; then
    actual=$(b3sum "${AVP_BIN}" | awk '{print $1}')
elif python3 -c "import blake3" >/dev/null 2>&1; then
    actual=$(python3 -c "import sys, blake3; sys.stdout.write(blake3.blake3(open('${AVP_BIN}','rb').read()).hexdigest())")
else
    echo "::error::install-avp.sh: neither b3sum nor python3-blake3 available" >&2
    exit 70
fi
expected=$(awk '{print $1}' "${AVP_BIN}.blake3" | head -1)
if [[ "${actual}" != "${expected}" ]]; then
    echo "::error::install-avp.sh: blake3 mismatch (expected=${expected} actual=${actual})" >&2
    exit 70
fi
echo "blake3 ok: ${actual}"
echo "::endgroup::"

# Sanity-print the version.
"${AVP_BIN}" --version
