#!/usr/bin/env python3
"""Recipe extractor — the Lisa SDK flagship (PLAN §5.6 acceptance).

Paste free text, get a typed Recipe via guided generation — the output is
grammar-constrained server-side, so it always parses. Works against
`lisa-inferenced` with the stock OpenAI client (no Lisa SDK needed): the
OpenAI-compat endpoint is the zero-dependency path.

    pip install openai
    python recipe_extractor.py
"""
import json
from dataclasses import dataclass
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:7777/v1", api_key="local")

SCHEMA = {
    "type": "object",
    "properties": {
        "title": {"type": "string", "maxLength": 80},
        "servings": {"type": "integer"},
        "ingredients": {"type": "array", "items": {"type": "string", "maxLength": 60},
                        "minItems": 1, "maxItems": 30},
        "steps": {"type": "array", "items": {"type": "string", "maxLength": 200},
                  "minItems": 1, "maxItems": 20},
    },
    "required": ["title", "servings", "ingredients", "steps"],
}

@dataclass
class Recipe:
    title: str; servings: int; ingredients: list[str]; steps: list[str]

def extract(text: str) -> Recipe:
    r = client.chat.completions.create(
        model="lisa", max_tokens=800,
        messages=[{"role": "user", "content": f"Extract the recipe.\n\n{text}"}],
        response_format={"type": "json_schema",
                         "json_schema": {"name": "recipe", "schema": SCHEMA}})
    return Recipe(**json.loads(r.choices[0].message.content))

if __name__ == "__main__":
    recipe = extract(
        "Grandma's lentil soup serves 6: red lentils, carrots, onion, cumin, "
        "and lemon. Sweat the onion and carrot, add lentils and cumin, cover "
        "with water, simmer 25 min, finish with lemon.")
    print(f"# {recipe.title}  (serves {recipe.servings})")
    print("\nIngredients:"); [print(f"  - {i}") for i in recipe.ingredients]
    print("\nSteps:"); [print(f"  {n}. {s}") for n, s in enumerate(recipe.steps, 1)]
