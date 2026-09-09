#!/usr/bin/env python3
"""Deterministic, input-aware curation provider for the okc full-flow demo.

Speaks the Ollama wire okc-core's `ollama` provider kind uses (`POST /api/embed`,
`POST /api/chat`) and returns VALID okc-core structured output over the REAL
uploaded documents — echoing the exact block/metadata ids + content hashes the
engine sent, which is why a deterministic provider satisfies okc-core's strict
synthesis/critic contracts where a live LLM does not.

What it produces (over the demo's near-duplicate note pairs):
  - organizer  : groups documents by filename stem → one topic cluster per stem
                 (so the two team versions of "환불 정책" etc. land together).
  - synthesis  : preserves every block/metadata verbatim, and for any multi-doc
                 cluster emits a PRESERVED CONTRADICTION ("승자 없음") between the
                 two team versions.
  - critic     : no blocking findings (contradictions are preserved, not errors),
                 so the review reaches a clean compile.

This is honestly synthetic (deterministic, not a live model) but it drives the
entire real pipeline: integrate → taxonomy → synthesis → critic → compile → serve.
"""
from __future__ import annotations

import json
import os
import re
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("CURATION_PORT", "8797"))


def _stem(path: str | None) -> str:
    return re.sub(r"\.md$", "", os.path.basename(path or "")) or "노트"


def _organizer(ti: dict) -> dict:
    groups: dict[str, list[str]] = {}
    order: list[str] = []
    for d in ti["documents"]:
        key = _stem(d.get("original_path"))
        if key not in groups:
            groups[key] = []
            order.append(key)
        groups[key].append(d["document_id"])
    clusters = []
    for i, key in enumerate(order, start=1):
        clusters.append({
            "cluster_id": f"topic-{i}",             # ASCII + URL-safe
            "title": key,                            # readable Korean stem
            "canonical_path": f"knowledge/topic-{i}.md",
            "document_ids": groups[key],
        })
    return {"clusters": clusters}


def _synthesis(ti: dict) -> dict:
    docs = ti["documents"]
    dispositions = []
    for d in docs:
        for b in d.get("blocks", []):
            dispositions.append({
                "kind": "block", "document_id": d["document_id"], "target_id": b["block_id"],
                "content_hash": b["content_hash"], "disposition": "preserved_verbatim", "rationale": "",
            })
        for m in d.get("metadata", []):
            dispositions.append({
                "kind": "metadata", "document_id": d["document_id"], "target_id": m["metadata_id"],
                "content_hash": m["content_hash"], "disposition": "preserved_verbatim", "rationale": "",
            })
    contradictions = []
    if len(docs) >= 2:
        claims = []
        for i, d in enumerate(docs[:2], start=1):
            b = (d.get("blocks") or [{}])[0]
            claims.append({
                "claim_id": f"claim-{i}",
                "rendered_claim": f"팀 문서 {i}의 주장 (문서 {d['document_id'][:12]}…)",
                "observed_at": "2026-09-09",
                "context": "같은 주제에 대한 팀별 문서",
                "evidence": [{
                    "document_id": d["document_id"],
                    "block_id": b.get("block_id", ""),
                    "content_hash": b.get("content_hash", ""),
                }],
            })
        contradictions.append({
            "contradiction_id": "contradiction-1",
            "summary": "두 팀 문서가 동일 주제에서 상반된 사실을 기술합니다. 승자를 고르지 않고 양측을 모두 보존합니다.",
            "claims": claims,
        })
    return {"sections": [], "related_links": [], "dispositions": dispositions, "contradictions": contradictions}


def _critic(_ti: dict) -> dict:
    # Contradictions are preserved by design (not synthesis errors), so the critic
    # raises no blocking findings — the review reaches a clean compile.
    return {"findings": []}


def _structured(ti: dict) -> dict:
    if "semantic_candidates" in ti:
        return _organizer(ti)
    if "proposal" in ti:
        return _critic(ti)
    return _synthesis(ti)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_a):  # quiet
        pass

    def do_POST(self):  # noqa: N802
        n = int(self.headers.get("content-length", "0") or "0")
        req = json.loads(self.rfile.read(n) or b"{}")
        if self.path == "/api/embed":
            inputs = req.get("input", [])
            resp = {"model": "curation", "embeddings": [[1, i + 1] for i in range(len(inputs))],
                    "prompt_eval_count": len(inputs)}
        elif self.path == "/api/chat":
            task_input = json.loads(req["messages"][1]["content"])
            resp = {"model": "curation", "done": True, "done_reason": "stop",
                    "prompt_eval_count": 1, "eval_count": 1,
                    "message": {"role": "assistant",
                                "content": json.dumps(_structured(task_input), ensure_ascii=False)}}
        else:
            self.send_error(404)
            return
        body = json.dumps(resp, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        self.close_connection = True


if __name__ == "__main__":
    print(f"[curation] deterministic provider on http://127.0.0.1:{PORT} (/api/embed, /api/chat)", flush=True)
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
