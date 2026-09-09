#!/usr/bin/env python3
"""scripts/ai-languages.py

List the language codes supported by the whisper model in --model-dir.

Lighter than loading the full model: only the tokenizer vocabulary is read to
detect language tokens (`<|xx|>`); a multilingual model exposes them, an
English-only model does not (only `en` applies).

Contract (called by the Rust wrapper):
  argv: --model-dir DIR
  stdout: one JSON object
      {"ok": true, "languages": ["en", "zh", "ja", ...]}
      {"ok": false, "error": "message"}
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

LANG_TOKEN_RE = re.compile(r"<\|([a-z]{2})\|>")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-dir", required=True)
    args = parser.parse_args()

    model_dir = Path(args.model_dir)

    # Tokenizer vocab: faster-whisper models ship vocabulary.txt
    # (one token per line) or tokenizer.json. Detect language tokens from
    # whichever exists.
    vocab_file = model_dir / "vocabulary.txt"
    tokenizer_file = model_dir / "tokenizer.json"

    codes: set[str] = set()
    try:
        if vocab_file.exists():
            text = vocab_file.read_text(encoding="utf-8", errors="ignore")
            codes |= set(LANG_TOKEN_RE.findall(text))
        elif tokenizer_file.exists():
            import json as _json

            data = _json.loads(tokenizer_file.read_text(encoding="utf-8"))
            vocab = data.get("model", {}).get("vocab", {})
            for token in vocab.keys():
                m = LANG_TOKEN_RE.match(token)
                if m:
                    codes.add(m.group(1))
    except Exception as exc:  # noqa: BLE001
        print(json.dumps({"ok": False, "error": str(exc)}, ensure_ascii=False))
        return 1

    # No language tokens found -> an English-only model.
    if not codes:
        codes = {"en"}

    print(json.dumps({"ok": True, "languages": sorted(codes)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
