#!/usr/bin/env python3
"""Regenerate codex-rs/models-manager/models.json from the live Fikra model list.

Usage: python3 scripts/fikra/gen-models.py

Only chat models priced per token are included. Metadata mirrors the fallback
used for unknown models (known to work against the Fikra Responses API), and
every entry embeds the Fikra Code base instructions.
"""
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2] / "codex-rs"
MODELS_URL = "https://fikraapi.co.ke/v1/models"
DEFAULT_MODEL = "qwen3.8-27b"
# Shown first in the picker, in this order. Everything else follows by name.
PREFERRED = [
    DEFAULT_MODEL,
    "fikra-auto",
    "fikra-auto-pro",
    "fikra-code-35b",
    "deepseek-v4.1-flash",
    "deepseek-v4-pro-pinned",
    "kimi-k3",
    "glm-5.3",
    "qwen4-max",
]
EXCLUDE_SUBSTRINGS = ("embedding", "tts", "stt", "seedream", "decision")
DESCRIPTIONS = {
    DEFAULT_MODEL: "Free: 200 requests and 2M tokens a day, no card needed.",
    "fikra-auto": "Fikra picks a fast, low-cost model for each request.",
    "fikra-auto-pro": "Fikra picks a frontier model for each request.",
    "fikra-code-35b": "Fikra coding model.",
}


def main() -> None:
    instructions = (ROOT / "models-manager" / "prompt.md").read_text()
    req = urllib.request.Request(MODELS_URL, headers={"User-Agent": "fikra-code-gen"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.load(resp)["data"]

    chat = []
    for m in data:
        mid = m["id"]
        pricing = m.get("pricing") or {}
        ctx = m.get("context_window") or 0
        if pricing.get("unit") != "USD_PER_1M_TOKENS" or ctx <= 0:
            continue
        if any(s in mid for s in EXCLUDE_SUBSTRINGS):
            continue
        chat.append(m)

    def order(m):
        mid = m["id"]
        return (PREFERRED.index(mid) if mid in PREFERRED else len(PREFERRED), mid)

    chat.sort(key=order)
    if not chat or chat[0]["id"] != DEFAULT_MODEL:
        raise SystemExit(f"default model {DEFAULT_MODEL} missing from {MODELS_URL}")

    models = []
    for priority, m in enumerate(chat):
        mid = m["id"]
        ctx = min(int(m["context_window"]), 1_000_000)
        models.append(
            {
                "slug": mid,
                "display_name": mid,
                "description": DESCRIPTIONS.get(mid, "Available through the Fikra API."),
                "default_reasoning_level": None,
                "supported_reasoning_levels": [],
                "shell_type": "unified_exec",
                "visibility": "list",
                "supported_in_api": True,
                "priority": priority,
                "additional_speed_tiers": [],
                "service_tiers": [],
                "default_service_tier": None,
                "availability_nux": None,
                "upgrade": None,
                "model_messages": {"instructions_template": instructions},
                "include_skills_usage_instructions": False,
                "include_plugin_usage_instructions": False,
                "include_apps_usage_instructions": False,
                "supports_reasoning_summary_parameter": True,
                "default_reasoning_summary": "auto",
                "support_verbosity": False,
                "default_verbosity": None,
                "apply_patch_tool_type": None,
                "web_search_tool_type": "text",
                "truncation_policy": {"mode": "bytes", "limit": 10000},
                "supports_image_detail_original": False,
                "context_window": ctx,
                "max_context_window": ctx,
                "auto_compact_token_limit": None,
                "experimental_supported_tools": [],
                "input_modalities": ["text"],
                "supports_search_tool": False,
                "supports_experimental_context": False,
                "use_responses_lite": False,
                "supports_reasoning_effort_updates": False,
                "guardian": None,
                "node_repl_auto_review_required": False,
                "node_repl_disabled": False,
                "auto_review_model_override": None,
                "model_specialty": None,
                "tool_mode": None,
                "multi_agent_version": None,
                "multi_agent_reasoning_effort": None,
            }
        )

    out = ROOT / "models-manager" / "models.json"
    out.write_text(json.dumps({"models": models}, indent=2) + "\n")
    print(f"wrote {len(models)} models to {out}")


if __name__ == "__main__":
    main()
