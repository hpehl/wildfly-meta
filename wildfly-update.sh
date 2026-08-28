#!/usr/bin/env bash
#
#  Copyright 2023 Red Hat
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      https://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
#


# -------------------------------------------------------
#
# Builds and pushes wado and mgt images for a new
# WildFly version.
#
# Prerequisites:
#   - Edit wildfly-images.toml with the new version
#     (bump config_version, add/update the entry)
#   - Commit the changes (do NOT push)
#   - podman (or docker) installed and running
#   - Logged in to quay.io/wado and quay.io/modelgraphtools
#   - wado and mgt CLIs installed
#
# -------------------------------------------------------

set -Eeuo pipefail
trap cleanup SIGINT SIGTERM ERR EXIT

VERSION=0.1.0

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)
readonly script_dir
cd "${script_dir}"

usage() {
  cat <<EOF
USAGE:
    $(basename "${BASH_SOURCE[0]}") [FLAGS] <wildfly-version>

FLAGS:
    -h, --help          Prints help information
    -v, --version       Prints version information
    --dry-run           Run everything except push steps
    --skip-wado         Skip wado build and push
    --skip-mgt          Skip mgt analyze and push
    --no-color          Uses plain text output

ARGS:
    <wildfly-version>   The WildFly version (e.g. 42, 26.1)

PREREQUISITES:
    Before running this script:
      1. Edit wildfly-images.toml:
         - Bump config_version
         - Add a new entry or update the last entry
      2. Commit the changes (do NOT push — the script handles that)
EOF
  exit
}

cleanup() {
  trap - SIGINT SIGTERM ERR EXIT
}

setup_colors() {
  if [[ -t 2 ]] && [[ -z "${NO_COLOR-}" ]] && [[ "${TERM-}" != "dumb" ]]; then
    NOFORMAT='\033[0m' RED='\033[0;31m' GREEN='\033[0;32m' ORANGE='\033[0;33m' BLUE='\033[0;34m' PURPLE='\033[0;35m' CYAN='\033[0;36m' YELLOW='\033[1;33m'
  else
    # shellcheck disable=SC2034
    NOFORMAT='' RED='' GREEN='' ORANGE='' BLUE='' PURPLE='' CYAN='' YELLOW=''
  fi
}

msg() {
  echo >&2 -e "${1-}"
}

die() {
  local msg=$1
  local code=${2-1}
  msg "${RED}ERROR: ${msg}${NOFORMAT}"
  exit "$code"
}

version() {
  msg "${BASH_SOURCE[0]} $VERSION"
  exit 0
}

DRY_RUN=false
SKIP_WADO=false
SKIP_MGT=false

parse_params() {
  while :; do
    case "${1-}" in
    -h | --help) usage ;;
    -v | --version) version ;;
    --dry-run) DRY_RUN=true ;;
    --skip-wado) SKIP_WADO=true ;;
    --skip-mgt) SKIP_MGT=true ;;
    --no-color) NO_COLOR=1 ;;
    -?*) die "Unknown option: $1" ;;
    *) break ;;
    esac
    shift
  done

  ARGS=("$@")
  [[ ${#ARGS[@]} -eq 1 ]] || die "Missing WildFly version"
  WILDFLY_VERSION=${ARGS[0]}
  return 0
}

parse_params "$@"
setup_colors

# ------------------------------------------------------ preflight checks

msg ""
msg "${BLUE}Preflight checks${NOFORMAT}"
msg ""

# Check required tools
for cmd in wado mgt; do
  command -v "${cmd}" &>/dev/null || die "'${cmd}' not found. Please install it first."
done

# Check container runtime
if command -v podman &>/dev/null; then
  CONTAINER_CMD="podman"
elif command -v docker &>/dev/null; then
  CONTAINER_CMD="docker"
else
  die "Neither 'podman' nor 'docker' found. Please install a container runtime."
fi
msg "Container runtime: ${CYAN}${CONTAINER_CMD}${NOFORMAT}"

# Check that wildfly-images.toml has the requested version
if ! grep -q "version = \"${WILDFLY_VERSION}" wildfly-images.toml; then
  die "Version ${WILDFLY_VERSION} not found in wildfly-images.toml. Please update the file first."
fi
msg "Version ${CYAN}${WILDFLY_VERSION}${NOFORMAT} found in wildfly-images.toml"

# Check that config_version was bumped (compare working tree against origin/main)
LOCAL_CONFIG_VERSION=$(grep -m1 '^config_version' wildfly-images.toml | grep -o '[0-9]*')
REMOTE_CONFIG_VERSION=$(git show origin/main:wildfly-images.toml 2>/dev/null | grep -m1 '^config_version' | grep -o '[0-9]*' || echo "0")
if [[ "${LOCAL_CONFIG_VERSION}" -le "${REMOTE_CONFIG_VERSION}" ]]; then
  die "config_version (${LOCAL_CONFIG_VERSION}) was not bumped. Please increment it in wildfly-images.toml."
fi
msg "config_version bumped: ${REMOTE_CONFIG_VERSION} → ${CYAN}${LOCAL_CONFIG_VERSION}${NOFORMAT}"

# Check for uncommitted changes to the TOML
if ! git diff --quiet wildfly-images.toml 2>/dev/null || ! git diff --cached --quiet wildfly-images.toml 2>/dev/null; then
  die "wildfly-images.toml has uncommitted changes. Please commit first."
fi
msg "wildfly-images.toml is committed"

# Check that there are unpushed commits (the script handles the push)
UNPUSHED=$(git log origin/main..HEAD --oneline 2>/dev/null | wc -l | tr -d ' ')
if [[ "${UNPUSHED}" -eq 0 ]]; then
  die "No unpushed commits found. The TOML changes should be committed but not yet pushed."
fi
msg "${CYAN}${UNPUSHED}${NOFORMAT} unpushed commit(s) — script will push in step 1"

# ------------------------------------------------------ summary

msg ""
msg "${BLUE}Plan${NOFORMAT}"
msg ""
if [[ "${DRY_RUN}" == true ]]; then
  msg "${YELLOW}DRY RUN${NOFORMAT} — push steps will be skipped"
fi
msg "WildFly version:  ${CYAN}${WILDFLY_VERSION}${NOFORMAT}"
[[ "${SKIP_WADO}" == true ]] && msg "wado:             ${YELLOW}skipped${NOFORMAT}" || msg "wado:             build and push"
[[ "${SKIP_MGT}" == true ]]  && msg "mgt:              ${YELLOW}skipped${NOFORMAT}" || msg "mgt:              analyze and push"
msg ""

echo "Do you wish to continue?"
select yn in "Yes" "No"; do
  case $yn in
    Yes ) break;;
    No ) die "Aborted" ;;
  esac
done

# ------------------------------------------------------ push TOML changes

msg ""
msg "${BLUE}Step 1: Push TOML changes${NOFORMAT}"
msg ""

if [[ "${DRY_RUN}" == true ]]; then
  msg "${YELLOW}DRY RUN: Skipping git push${NOFORMAT}"
else
  git push
  msg "Pushed to origin"
  msg "Waiting for GitHub CDN to update..."
  sleep 10
fi

# ------------------------------------------------------ update metadata

msg ""
msg "${BLUE}Step 2: Update local metadata${NOFORMAT}"
msg ""

wado update --json
msg "wado metadata updated"
mgt update --json
msg "mgt metadata updated"

# ------------------------------------------------------ wado

if [[ "${SKIP_WADO}" == false ]]; then
  msg ""
  msg "${BLUE}Step 3: Build and push wado images${NOFORMAT}"
  msg ""

  msg "Stopping all running wado containers..."
  wado stop --all --json || true

  msg "Building wado images for ${WILDFLY_VERSION}..."
  wado build "${WILDFLY_VERSION}" --verbose

  if [[ "${DRY_RUN}" == true ]]; then
    msg "${YELLOW}DRY RUN: Skipping wado push${NOFORMAT}"
  else
    msg "Pushing wado images for ${WILDFLY_VERSION}..."
    wado push "${WILDFLY_VERSION}" --json
  fi
fi

# ------------------------------------------------------ mgt

if [[ "${SKIP_MGT}" == false ]]; then
  msg ""
  msg "${BLUE}Step 4: Analyze and push mgt images${NOFORMAT}"
  msg ""

  msg "Stopping all running wado containers..."
  wado stop --all --json || true

  msg "Analyzing ${WILDFLY_VERSION}..."
  mgt analyze "${WILDFLY_VERSION}" --json

  if [[ "${DRY_RUN}" == true ]]; then
    msg "${YELLOW}DRY RUN: Skipping mgt push${NOFORMAT}"
  else
    msg "Pushing mgt images for ${WILDFLY_VERSION}..."
    mgt push "${WILDFLY_VERSION}" --json
  fi
fi

# ------------------------------------------------------ done

msg ""
msg "${GREEN}Done!${NOFORMAT}"
if [[ "${DRY_RUN}" == true ]]; then
  msg "${YELLOW}This was a dry run. No images were pushed.${NOFORMAT}"
fi
msg ""
