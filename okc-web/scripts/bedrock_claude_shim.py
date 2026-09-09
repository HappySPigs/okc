#!/usr/bin/env python3
"""Localhost shim: okc-core `anthropic` provider kind -> AWS Bedrock Converse.

okc-core has no native Bedrock transport. Its `anthropic` kind speaks the
Anthropic Messages wire format (`POST {endpoint}/v1/messages`, structured output
via `output_config.format.json_schema`). This process accepts that request on
127.0.0.1 and forwards it to Bedrock's native Converse API with the account's
Bedrock API key (Bearer), mapping the requested json-schema onto a Converse
forced-tool call and returning an Anthropic-Messages-shaped response that
okc-core's parser accepts.

Generation only. Embeddings are intentionally unsupported (this key/policy
cannot do Bedrock embeddings, and okc-core's anthropic kind never embeds).

Env:
  AWS_BEARER_TOKEN_BEDROCK  required  Bedrock API key (ABSK...)
  AWS_REGION                required  e.g. ap-northeast-2
  BEDROCK_MODEL             required  e.g. global.anthropic.claude-sonnet-5
  SHIM_PORT                 optional  default 8799
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TOKEN = os.environ.get("AWS_BEARER_TOKEN_BEDROCK", "")
REGION = os.environ.get("AWS_REGION", "")
MODEL = os.environ.get("BEDROCK_MODEL", "")
PORT = int(os.environ.get("SHIM_PORT", "8799"))
_TOOL = "structured_output"


def _converse_url(model: str) -> str:
    return f"https://bedrock-runtime.{REGION}.amazonaws.com/model/{model}/converse"


def _to_converse(req: dict) -> dict:
    """Anthropic-Messages request (as okc-core sends it) -> Bedrock Converse."""
    messages = []
    for m in req.get("messages", []):
        content = m.get("content", "")
        blocks = [{"text": content}] if isinstance(content, str) else [
            {"text": c["text"]} if isinstance(c, dict) and "text" in c else {"text": str(c)}
            for c in content
        ]
        messages.append({"role": m.get("role", "user"), "content": blocks})

    inference: dict = {}
    if isinstance(req.get("max_tokens"), int):
        # Floor the cap: newer Claude models spend a few tokens before emitting
        # the forced tool call, so a tiny okc cap (e.g. the 32-token capability
        # probe) truncates it to `{}`. maxTokens is only an upper bound, so
        # raising a small one is harmless; real tasks already request more.
        inference["maxTokens"] = max(req["max_tokens"], 1024)
    # `temperature` is intentionally not forwarded: newer Claude models on
    # Bedrock (e.g. sonnet-5) reject it as deprecated, and forced-tool
    # structured output does not need it.

    body: dict = {"messages": messages}
    system = req.get("system")
    if isinstance(system, str) and system.strip():
        body["system"] = [{"text": system}]
    if inference:
        body["inferenceConfig"] = inference

    # Structured output -> forced tool call carrying the json schema.
    schema = (req.get("output_config") or {}).get("format", {}).get("schema")
    if isinstance(schema, dict):
        body["toolConfig"] = {
            "tools": [{"toolSpec": {
                "name": _TOOL,
                "description": "Return the result strictly as this JSON object.",
                "inputSchema": {"json": schema},
            }}],
            "toolChoice": {"tool": {"name": _TOOL}},
        }
    return body


def _unwrap_tool_input(obj: object) -> object:
    """Bedrock's Converse forced-tool sometimes returns the payload double-encoded:
    the whole object arrives as a JSON *string* nested under the top-level key(s),
    e.g. ``{"clusters": "{\\"clusters\\":[...]}"}``. Undo that so okc-core sees the
    real types. Idempotent for already-correct inputs."""
    for _ in range(4):  # unwrap a few levels defensively
        if not isinstance(obj, dict):
            return obj
        changed = False
        # whole payload stringified under a top key: {"k": "<json with k>"}
        if len(obj) == 1:
            (k, v), = obj.items()
            if isinstance(v, str):
                try:
                    parsed = json.loads(v)
                except (ValueError, TypeError):
                    parsed = None
                if isinstance(parsed, dict) and k in parsed:
                    obj = parsed
                    continue
        # individual values stringified: {"k": "[...]" or "{...}"}
        for k, v in list(obj.items()):
            if isinstance(v, str) and v[:1] in ("[", "{"):
                try:
                    obj[k] = json.loads(v)
                    changed = True
                except (ValueError, TypeError):
                    pass
        if not changed:
            return obj
    return obj


def _from_converse(model: str, wire: dict) -> dict:
    """Bedrock Converse response -> Anthropic-Messages shape okc-core parses."""
    blocks = (wire.get("output", {}).get("message", {}) or {}).get("content", []) or []
    text = None
    for b in blocks:
        if "toolUse" in b:  # structured path: serialize the tool input object
            text = json.dumps(_unwrap_tool_input(b["toolUse"].get("input", {})), ensure_ascii=False)
            break
    if text is None:  # fallback: concatenate any plain text blocks
        text = "".join(b.get("text", "") for b in blocks if "text" in b)

    usage = wire.get("usage", {}) or {}
    return {
        "model": model,
        "stop_reason": "end_turn",  # okc-core only rejects "refusal"
        "content": [{"type": "text", "text": text}],
        "usage": {
            "input_tokens": usage.get("inputTokens"),
            "output_tokens": usage.get("outputTokens"),
        },
    }


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_a):  # keep stdout quiet; we log explicitly
        pass

    def _send(self, code: int, payload: dict) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self) -> None:  # noqa: N802
        if self.path.rstrip("/") not in ("/v1/messages", ""):
            self._send(404, {"type": "error", "error": {"message": f"no route {self.path}"}})
            return
        raw = self.rfile.read(int(self.headers.get("Content-Length", "0") or "0"))
        try:
            req = json.loads(raw or b"{}")
        except json.JSONDecodeError as e:
            self._send(400, {"type": "error", "error": {"message": f"bad json: {e}"}})
            return

        model = req.get("model") if isinstance(req.get("model"), str) and req.get("model", "").startswith(("global.", "apac.", "anthropic.")) else MODEL
        converse = _to_converse(req)
        http_req = urllib.request.Request(
            _converse_url(model),
            data=json.dumps(converse).encode("utf-8"),
            headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(http_req, timeout=120) as resp:
                wire = json.loads(resp.read())
            self._send(200, _from_converse(model, wire))
            print(f"[shim] 200 model={model} in={wire.get('usage',{}).get('inputTokens')} out={wire.get('usage',{}).get('outputTokens')}", flush=True)
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", "replace")[:800]
            print(f"[shim] bedrock {e.code}: {detail}", flush=True)
            # Surface upstream status so okc-core maps it to a typed provider error.
            self._send(e.code, {"type": "error", "error": {"message": detail}})
        except Exception as e:  # noqa: BLE001
            print(f"[shim] error: {e}", flush=True)
            self._send(502, {"type": "error", "error": {"message": str(e)}})


def main() -> int:
    missing = [k for k, v in (("AWS_BEARER_TOKEN_BEDROCK", TOKEN), ("AWS_REGION", REGION), ("BEDROCK_MODEL", MODEL)) if not v]
    if missing:
        print(f"[shim] missing env: {', '.join(missing)}", file=sys.stderr)
        return 2
    srv = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print(f"[shim] listening on http://127.0.0.1:{PORT}/v1/messages -> Bedrock Converse "
          f"({REGION}, model={MODEL})", flush=True)
    srv.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
