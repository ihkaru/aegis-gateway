#!/usr/bin/env bash
set -uo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Run the comprehensive governance check
if OUTPUT=$(bash scripts/governance-check.sh 2>&1); then
  echo '{"decision": "allow"}'
  exit 0
else
  # Escape double quotes and newlines for JSON payload
  ESCAPED_OUTPUT=$(echo "$OUTPUT" | tail -n 15 | tr '\n' ' ' | sed 's/"/\\"/g')
  echo "{\"decision\": \"continue\", \"reason\": \"Aegis Enterprise Governance check FAILED! Coding agent must remediate violations before stopping: $ESCAPED_OUTPUT\"}"
  exit 0
fi
