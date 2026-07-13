"""
GNN Inference Server for X-MaC
Receives file system graphs and returns safety/anomaly scores.
Can run standalone (Docker) or be replaced by CoreML on-device.
"""

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
import torch
import json
import time
import os
import traceback

from model.gnn import GATModel

app = FastAPI(title="X-MaC GNN Inference Server", version="0.2.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost", "http://127.0.0.1"],
    allow_methods=["POST"],
    allow_headers=["*"],
)

# Load model
MODEL_PATH = os.environ.get("XMAC_MODEL_PATH", "/app/model/xmac_gnn.pt")
LABEL_MAP_PATH = os.environ.get("XMAC_LABEL_MAP", "/app/label_map.json")

device = torch.device("cpu")
model = None
label_map = {}
reverse_label_map = {}

def load_model():
    global model, label_map, reverse_label_map
    if os.path.exists(MODEL_PATH):
        checkpoint = torch.load(MODEL_PATH, map_location=device, weights_only=True)
        num_features = checkpoint.get("num_features", 16)
        num_classes = checkpoint.get("num_classes", 27)
        hidden_dim = checkpoint.get("hidden_dim", 128)
        model = GATModel(
            num_features=num_features,
            hidden_dim=hidden_dim,
            num_classes=num_classes,
            num_heads=checkpoint.get("num_heads", 8),
            num_layers=checkpoint.get("num_layers", 3),
            dropout=checkpoint.get("dropout", 0.2),
        )
        model.load_state_dict(checkpoint["model_state_dict"])
        model.eval()
        print(f"Model loaded from {MODEL_PATH}")
    else:
        print(f"WARNING: Model not found at {MODEL_PATH}")

    if os.path.exists(LABEL_MAP_PATH):
        with open(LABEL_MAP_PATH) as f:
            label_map = json.load(f)
        reverse_label_map = {v: k for k, v in label_map.items()}

load_model()


class GraphRequest(BaseModel):
    nodes: List[Dict[str, Any]]
    edges: List[Dict[str, Any]]
    root_path: str = ""
    num_features: int = 16


class ScoreResponse(BaseModel):
    scores: List[Dict[str, Any]]
    summary: Optional[Dict[str, Any]] = None
    purge_plan: Optional[Dict[str, Any]] = None
    timing: Optional[Dict[str, Any]] = None


@app.get("/health")
async def health():
    return {"status": "ok", "model_loaded": model is not None}


@app.post("/predict", response_model=ScoreResponse)
async def predict(req: GraphRequest):
    if model is None:
        raise HTTPException(status_code=503, detail="Model not loaded")

    t0 = time.time()

    # Build tensors
    node_features = torch.tensor(
        [n.get("features", [0.0] * req.num_features) for n in req.nodes],
        dtype=torch.float32
    ).unsqueeze(0)  # [1, N, F]

    if req.edges:
        edge_src = [e["source"] for e in req.edges]
        edge_dst = [e["target"] for e in req.edges]
        edge_index = torch.tensor([edge_src, edge_dst], dtype=torch.long)
    else:
        edge_index = torch.empty((2, 0), dtype=torch.long)

    t1 = time.time()

    # Run inference
    with torch.no_grad():
        logits, safety_logits, anomaly_logits = model(node_features, edge_index)
        safety_scores = torch.sigmoid(safety_logits.squeeze(0).squeeze(-1))
        anomaly_scores = torch.sigmoid(anomaly_logits.squeeze(0).squeeze(-1))

    t2 = time.time()

    # Build response
    scores = []
    for i, node in enumerate(req.nodes):
        safety = safety_scores[i].item()
        anomaly = anomaly_scores[i].item()
        label_idx = int(torch.argmax(logits[0, i]).item())
        label = reverse_label_map.get(label_idx, "file")

        scores.append({
            "path": node.get("path", ""),
            "label": label,
            "safety_score": safety,
            "anomaly_score": anomaly,
            "size_bytes": node.get("size_bytes", 0),
        })

    # Summary
    safe_count = sum(1 for s in scores if s["safety_score"] >= 0.7)
    review_count = sum(1 for s in scores if 0.4 <= s["safety_score"] < 0.7)
    danger_count = sum(1 for s in scores if s["safety_score"] < 0.4)
    avg_safety = sum(s["safety_score"] for s in scores) / max(len(scores), 1)
    avg_anomaly = sum(s["anomaly_score"] for s in scores) / max(len(scores), 1)
    potential_reclaim = sum(s["size_bytes"] for s in scores if s["safety_score"] >= 0.7)

    summary = {
        "total_files": len(scores),
        "safe_files": safe_count,
        "review_files": review_count,
        "danger_files": danger_count,
        "avg_safety": avg_safety,
        "avg_anomaly": avg_anomaly,
        "potential_reclaim_bytes": potential_reclaim,
    }

    # Purge plan
    purge_plan = build_purge_plan(scores)

    t3 = time.time()

    return ScoreResponse(
        scores=scores,
        summary=summary,
        purge_plan=purge_plan,
        timing={
            "parse_ms": (t1 - t0) * 1000,
            "inference_ms": (t2 - t1) * 1000,
            "total_ms": (t3 - t0) * 1000,
        }
    )


def build_purge_plan(scores):
    # Impact-weighted ordering
    impact_weighted = sorted(
        [s for s in scores if s["safety_score"] >= 0.5],
        key=lambda s: s["safety_score"] * s.get("size_bytes", 0),
        reverse=True
    )[:50]

    # Anomaly hotspots
    dir_anomalies = {}
    for s in scores:
        if s["anomaly_score"] > 0.5:
            d = os.path.dirname(s["path"])
            if d not in dir_anomalies:
                dir_anomalies[d] = {"count": 0, "total": 0.0}
            dir_anomalies[d]["count"] += 1
            dir_anomalies[d]["total"] += s["anomaly_score"]

    hotspots = sorted(
        [{"directory": d, "anomaly_count": v["count"], "avg_anomaly": v["total"] / v["count"]}
         for d, v in dir_anomalies.items() if v["count"] >= 2],
        key=lambda x: x["anomaly_count"],
        reverse=True
    )[:10]

    # Cleanup confidence
    safe_ratio = sum(1 for s in scores if s["safety_score"] >= 0.7) / max(len(scores), 1)
    if safe_ratio > 0.7:
        confidence = "VERY HIGH"
    elif safe_ratio > 0.5:
        confidence = "HIGH"
    elif safe_ratio > 0.3:
        confidence = "MODERATE"
    else:
        confidence = "LOW"

    # Cross-directory patterns
    pattern_groups = {}
    for s in scores:
        if s["safety_score"] >= 0.5:
            key = s["label"]
            if key not in pattern_groups:
                pattern_groups[key] = {"count": 0, "total_size": 0, "paths": []}
            pattern_groups[key]["count"] += 1
            pattern_groups[key]["total_size"] += s.get("size_bytes", 0)
            if len(pattern_groups[key]["paths"]) < 5:
                pattern_groups[key]["paths"].append(s["path"])

    patterns = sorted(
        [{"pattern": k, "file_count": v["count"], "total_size": v["total_size"], "example_paths": v["paths"]}
         for k, v in pattern_groups.items() if v["count"] >= 3],
        key=lambda x: x["file_count"],
        reverse=True
    )[:10]

    return {
        "impact_weighted_order": [
            {"path": s["path"], "safety_score": s["safety_score"],
             "size_bytes": s.get("size_bytes", 0),
             "impact_score": s["safety_score"] * s.get("size_bytes", 0)}
            for s in impact_weighted
        ],
        "anomaly_hotspots": hotspots,
        "cleanup_confidence": confidence,
        "cross_directory_patterns": patterns,
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8501)
