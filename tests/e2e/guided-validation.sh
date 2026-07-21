#!/usr/bin/env bash
# Guided-generation validation gate (PLAN §5.1 acceptance: given a JSON
# Schema, N sampled outputs → 100% parse + validate; N=1000 for the
# gate). Requires a running lisa-inferenced with a real model.
#   SAMPLES=25 tests/e2e/guided-validation.sh
set -euo pipefail

SAMPLES=${SAMPLES:-1000}
URL=${LISA_INFERENCE_URL:-http://127.0.0.1:7777}
SCHEMA='{"type":"object","properties":{"title":{"type":"string","maxLength":80},"servings":{"type":"integer"},"vegetarian":{"type":"boolean"},"ingredients":{"type":"array","items":{"type":"string","maxLength":60},"minItems":1,"maxItems":12}},"required":["title","servings","vegetarian","ingredients"]}'

PROMPTS=(
  "Extract the recipe: lentil soup for six, vegetarian, with lentils and cumin."
  "Extract the recipe: beef stew for two with beef, potatoes, and thyme."
  "Extract the recipe: pancakes for four - flour, milk, eggs, butter."
  "Extract the recipe: a salad for one, vegan, with kale, walnuts, apple."
  "Extract the recipe: chicken curry for five - chicken, rice, curry paste, coconut milk."
)

fail=0
for i in $(seq 1 "$SAMPLES"); do
  prompt=${PROMPTS[$(( (i - 1) % ${#PROMPTS[@]} ))]}
  out=$(curl -sf "$URL/v1/chat/completions" -H 'Content-Type: application/json' -d "$(python3 - "$prompt" "$SCHEMA" <<'PYEOF'
import json, sys
print(json.dumps({
    "messages": [{"role": "user", "content": sys.argv[1]}],
    "max_tokens": 512,
    "response_format": {"type": "json_schema",
                        "json_schema": {"name": "recipe", "schema": json.loads(sys.argv[2])}},
}))
PYEOF
)")
  content=$(python3 -c "
import json, sys
r = json.loads(sys.argv[1])
print(r['choices'][0]['message']['content'])
" "$out")
  if ! python3 -c "
import json, sys
d = json.loads(sys.argv[1])
assert isinstance(d['title'], str) and d['title']
assert isinstance(d['servings'], int) and not isinstance(d['servings'], bool)
assert isinstance(d['vegetarian'], bool)
assert isinstance(d['ingredients'], list) and d['ingredients']
assert all(isinstance(x, str) for x in d['ingredients'])
" "$content" 2>/dev/null; then
    fail=$((fail + 1))
    echo "INVALID sample $i: $content" >&2
  fi
  [ $((i % 50)) -eq 0 ] && echo "  $i/$SAMPLES, $fail invalid"
done

echo "GUIDED VALIDATION: $((SAMPLES - fail))/$SAMPLES valid"
[ "$fail" -eq 0 ] || exit 1
