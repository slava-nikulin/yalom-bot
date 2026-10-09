
#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

MODE="on_demand"
MODEL="gpt-6-luna"
EFFORT="low"
TIMEZONE="Europe/Moscow"
COUNT=1
CONTEXT=""
RAW=false
DRY_RUN=false

usage() {
  cat <<'EOF'
Usage: ./test.sh [options]

Options:
  -m, --mode MODE       on_demand | scheduled
      --model MODEL     OpenAI model ID
  -e, --effort EFFORT   default | none | low | medium | high | xhigh | max
  -z, --tz TIMEZONE     IANA timezone
  -n, --count N         Number of independent generations
      --context TEXT    Additional user context
      --raw             Print complete API response
      --dry-run         Print request JSON without calling API
  -h, --help            Show this help
EOF
}

while (($#)); do
  case "$1" in
    -m|--mode) MODE="${2:?Missing mode}"; shift 2 ;;
    --model) MODEL="${2:?Missing model}"; shift 2 ;;
    -e|--effort) EFFORT="${2:?Missing effort}"; shift 2 ;;
    -z|--tz) TIMEZONE="${2:?Missing timezone}"; shift 2 ;;
    -n|--count) COUNT="${2:?Missing count}"; shift 2 ;;
    --context) CONTEXT="${2:?Missing context}"; shift 2 ;;
    --raw) RAW=true; shift ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

case "$MODE" in
  on_demand|scheduled) ;;
  *) echo "Invalid mode: $MODE" >&2; exit 1 ;;
esac

case "$EFFORT" in
  default|none|low|medium|high|xhigh|max) ;;
  *) echo "Invalid effort: $EFFORT" >&2; exit 1 ;;
esac

[[ "$COUNT" =~ ^[1-9][0-9]*$ ]] || {
  echo "Count must be a positive integer" >&2
  exit 1
}

for cmd in curl jq date; do
  command -v "$cmd" >/dev/null || {
    echo "Missing dependency: $cmd" >&2
    exit 1
  }
done

[[ -f "/usr/share/zoneinfo/$TIMEZONE" ]] || {
  echo "Unknown timezone: $TIMEZONE" >&2
  exit 1
}

NOW=$(TZ="$TIMEZONE" date '+%Y-%m-%dT%H:%M:%S%:z')

INSTRUCTIONS=$(
  cat "$DIR/identity.md"
  printf '\n\n'
  cat "$DIR/examples.md"
  printf '\n\n'
  cat "$DIR/$MODE.md"
)

USER_CONTEXT=$(printf \
  'Timezone: %s\nCurrent local datetime: %s\n%s' \
  "$TIMEZONE" "$NOW" "$CONTEXT")

REQUEST=$(mktemp)
RESPONSE=$(mktemp)
trap 'rm -f "$REQUEST" "$RESPONSE"' EXIT

# Build request from templates.
jq \
  --arg model "$MODEL" \
  --arg effort "$EFFORT" \
  --arg mode "$MODE" \
  --arg instructions "$INSTRUCTIONS" \
  --arg context "$USER_CONTEXT" \
  --slurpfile schema "$DIR/scheduled.schema.json" \
  '
  .model = $model
  | .instructions = $instructions
  | .input[0].content = $context
  | if $effort == "default"
      then del(.reasoning)
      else .reasoning = {"effort": $effort}
    end
  | if $mode == "scheduled"
      then .text = {
        "format": {
          "type": "json_schema",
          "name": "scheduled_nudge",
          "strict": true,
          "schema": $schema[0]
        }
      }
      else del(.text)
    end
  ' "$DIR/request.json" > "$REQUEST"

if "$DRY_RUN"; then
  cat "$REQUEST"
  exit 0
fi

# Read API key once. Never write it to disk.
if [[ -z "${OPENAI_API_KEY:-}" ]]; then
  read -r -s -p "OpenAI API key: " OPENAI_API_KEY </dev/tty
  printf '\n' >&2
fi

[[ -n "$OPENAI_API_KEY" ]] || {
  echo "OPENAI_API_KEY is empty" >&2
  exit 1
}

for ((i = 1; i <= COUNT; i++)); do
  printf '\n--- %s | %s | %s | %d/%d ---\n' \
    "$MODE" "$MODEL" "$EFFORT" "$i" "$COUNT"

  
  if ! LATENCY=$(curl -sS --fail-with-body \
    --write-out '%{time_total}' \
    https://api.openai.com/v1/responses \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -H "Content-Type: application/json" \
    --data-binary @"$REQUEST" \
    -o "$RESPONSE"); then

    jq . "$RESPONSE" >&2 || cat "$RESPONSE" >&2
    exit 1
  fi

  if "$RAW"; then
    jq . "$RESPONSE"
    continue
  fi

  if ! jq -e '.status == "completed"' "$RESPONSE" >/dev/null; then
    jq '{status, error, incomplete_details}' "$RESPONSE" >&2
    exit 1
  fi

  TEXT=$(jq -r '
    [
      .output[]?
      | select(.type == "message")
      | .content[]?
      | select(.type == "output_text")
      | .text
    ]
    | join("\n")
  ' "$RESPONSE")

  if [[ -z "$TEXT" ]]; then
    echo "Empty output. Retry with --raw" >&2
    exit 1
  fi

  if [[ "$MODE" == "scheduled" ]]; then
    jq . <<< "$TEXT"
  else
    printf '%s\n' "$TEXT"
  fi

  jq -r '
    "Tokens: input=\(.usage.input_tokens), " +
    "output=\(.usage.output_tokens), " +
    "reasoning=\(.usage.output_tokens_details.reasoning_tokens // 0)"
  ' "$RESPONSE" >&2

  printf 'Latency: %ss\n' "$LATENCY" >&2
done
