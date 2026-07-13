#!/usr/bin/env python3

import argparse
import json
import os
import time
from pathlib import Path

os.environ.setdefault("KMP_DUPLICATE_LIB_OK", "TRUE")

import numpy as np
import torch
from sklearn.metrics import classification_report, confusion_matrix
from torch_geometric.loader import DataLoader

from data_generator import CATEGORY_SPECS, DATA_DIR, IDX_TO_LABEL, LABEL_MAP, NUM_FEATURES
from model.gnn import GATModel

GNN_DIR = Path(__file__).resolve().parent
CHECKPOINT_PATH = GNN_DIR / "model" / "xmac_gnn.pt"


def load_model(checkpoint_path):
    checkpoint = torch.load(checkpoint_path, map_location="cpu", weights_only=True)
    model = GATModel(
        num_features=checkpoint.get("num_features", NUM_FEATURES),
        hidden_dim=checkpoint["hidden_dim"],
        num_classes=checkpoint.get("num_classes", len(LABEL_MAP)),
        num_heads=checkpoint["num_heads"],
        num_layers=checkpoint.get("num_layers", 3),
        dropout=checkpoint.get("dropout", 0.2),
    )
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()
    return model, checkpoint


def evaluate(model, graphs, batch_size):
    loader = DataLoader(graphs, batch_size=batch_size, shuffle=False)
    labels = []
    predictions = []
    safety_error = 0.0
    anomaly_error = 0.0
    total = 0
    with torch.no_grad():
        for batch in loader:
            logits, safety_logits, anomaly_logits = model(batch.x, batch.edge_index, batch.batch)
            predicted = logits.argmax(dim=1)
            safety = torch.sigmoid(safety_logits.squeeze(-1))
            anomaly = torch.sigmoid(anomaly_logits.squeeze(-1))
            labels.extend(batch.y.tolist())
            predictions.extend(predicted.tolist())
            safety_error += (safety - batch.safety).abs().sum().item()
            anomaly_error += (anomaly - batch.anomaly).abs().sum().item()
            total += batch.y.numel()
    labels = np.asarray(labels)
    predictions = np.asarray(predictions)
    return labels, predictions, safety_error / total, anomaly_error / total


def save_reports(labels, predictions, test_acc):
    names = [IDX_TO_LABEL[index] for index in range(len(LABEL_MAP))]
    report = classification_report(
        labels, predictions, labels=list(range(len(names))), target_names=names, digits=4, zero_division=0
    )
    (GNN_DIR / "classification_report.txt").write_text(report)
    matrix = confusion_matrix(labels, predictions, labels=list(range(len(names))))
    np.save(GNN_DIR / "confusion_matrix.npy", matrix)
    try:
        import matplotlib.pyplot as plt
        import seaborn as sns
        plt.figure(figsize=(18, 15))
        sns.heatmap(matrix, annot=True, fmt="d", xticklabels=names, yticklabels=names, cmap="Blues")
        plt.title(f"Confusion Matrix — Test Accuracy: {test_acc:.4f}")
        plt.xlabel("Predicted")
        plt.ylabel("True")
        plt.tight_layout()
        plt.savefig(GNN_DIR / "confusion_matrix.png", dpi=150)
        plt.close()
    except ImportError as error:
        print(f"Confusion heatmap unavailable: {error}")
    return report, matrix


def safety_boundary_confusion(labels, predictions):
    safe = {LABEL_MAP[name] for name, spec in CATEGORY_SPECS.items() if spec.safety >= 0.7}
    unsafe = {LABEL_MAP[name] for name, spec in CATEGORY_SPECS.items() if spec.safety < 0.4}
    boundary_total = sum(int(label in safe or label in unsafe) for label in labels)
    crossed = sum(int((label in safe and prediction in unsafe) or (label in unsafe and prediction in safe))
                  for label, prediction in zip(labels, predictions))
    return crossed / max(boundary_total, 1)


def latency_ms(model, graph, runs=20):
    model.eval()
    x = graph.x[:200]
    keep = (graph.edge_index[0] < 200) & (graph.edge_index[1] < 200)
    edges = graph.edge_index[:, keep]
    with torch.no_grad():
        for _ in range(5):
            model(x, edges)
        started = time.perf_counter()
        for _ in range(runs):
            model(x, edges)
    return (time.perf_counter() - started) * 1000 / runs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", type=Path, default=CHECKPOINT_PATH)
    parser.add_argument("--batch-size", type=int, default=64)
    args = parser.parse_args()
    model, checkpoint = load_model(args.checkpoint)
    # Training data uses PyG Data objects which require pickle.
    # TODO: migrate to safetensors to enable weights_only=True.
    graphs = torch.load(DATA_DIR / "test.pt", weights_only=False)
    labels, predictions, safety_mae, anomaly_mae = evaluate(model, graphs, args.batch_size)
    test_acc = float((labels == predictions).mean())
    report, matrix = save_reports(labels, predictions, test_acc)
    per_class = {}
    for index in range(len(LABEL_MAP)):
        mask = labels == index
        per_class[IDX_TO_LABEL[index]] = float((predictions[mask] == index).mean())
    worst_class = min(per_class.items(), key=lambda item: item[1])
    cache_config_rate = matrix[LABEL_MAP["cache_dir"], LABEL_MAP["config_file"]] / max(
        matrix[LABEL_MAP["cache_dir"]].sum(), 1
    )
    boundary_rate = safety_boundary_confusion(labels, predictions)
    latency = latency_ms(model, max(graphs, key=lambda graph: graph.num_nodes))
    results = {
        "checkpoint_epoch": checkpoint.get("epoch"),
        "validation_accuracy": checkpoint.get("val_metrics", {}).get("acc"),
        "test_accuracy": test_acc,
        "safety_mae": safety_mae,
        "anomaly_mae": anomaly_mae,
        "per_class_accuracy": per_class,
        "worst_class": {"name": worst_class[0], "accuracy": worst_class[1]},
        "cache_dir_to_config_file_confusion": float(cache_config_rate),
        "safe_unsafe_boundary_confusion": boundary_rate,
        "cpu_latency_ms_200_nodes": latency,
    }
    (GNN_DIR / "evaluation_results.json").write_text(json.dumps(results, indent=2))
    print(f"TEST ACCURACY: {test_acc:.4f}")
    print("PER-CLASS ACCURACY:")
    for name, accuracy in per_class.items():
        count = int((labels == LABEL_MAP[name]).sum())
        correct = int(((labels == LABEL_MAP[name]) & (predictions == LABEL_MAP[name])).sum())
        print(f"  {name:30s}  {accuracy:.4f}  ({correct}/{count})")
    print(f"Safety MAE: {safety_mae:.4f}")
    print(f"Anomaly MAE: {anomaly_mae:.4f}")
    print(f"Safe/unsafe boundary confusion: {boundary_rate:.4f}")
    print(f"cache_dir -> config_file confusion: {cache_config_rate:.4f}")
    print(f"CPU latency (200 nodes): {latency:.2f} ms")
    print(report)
    accepted = (
        test_acc >= 0.95 and worst_class[1] >= 0.85 and safety_mae <= 0.08 and anomaly_mae <= 0.10
        and cache_config_rate < 0.02 and boundary_rate < 0.02 and latency < 50.0
    )
    if not accepted:
        raise SystemExit("Acceptance criteria not met")


if __name__ == "__main__":
    main()
