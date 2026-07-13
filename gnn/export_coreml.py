#!/usr/bin/env python3

import argparse
import json
import os
import random
from pathlib import Path

os.environ.setdefault("KMP_DUPLICATE_LIB_OK", "TRUE")

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, TensorDataset

from data_generator import DATA_DIR, NUM_FEATURES
from evaluate import load_model

GNN_DIR = Path(__file__).resolve().parent
COREML_EXPORT_PATH = GNN_DIR / "XMacGNN.mlpackage"
CHECKPOINT_PATH = GNN_DIR / "model" / "xmac_gnn.pt"
DISTILL_CHECKPOINT_PATH = GNN_DIR / "model" / "xmac_node_mlp.pt"
NUM_CLASSES = 27


class NodeMLP(nn.Module):
    def __init__(self, in_features=NUM_FEATURES, hidden=128, num_classes=NUM_CLASSES):
        super().__init__()
        self.fc1 = nn.Linear(in_features, hidden)
        self.fc2 = nn.Linear(hidden, hidden)
        self.fc3 = nn.Linear(hidden, hidden)
        self.norm1 = nn.LayerNorm(hidden)
        self.norm2 = nn.LayerNorm(hidden)
        self.safety_head = nn.Sequential(nn.Linear(hidden, hidden), nn.ReLU(), nn.Dropout(0.1), nn.Linear(hidden, 1))
        self.anomaly_head = nn.Sequential(nn.Linear(hidden, hidden), nn.ReLU(), nn.Dropout(0.1), nn.Linear(hidden, 1))
        self.class_head = nn.Sequential(nn.Linear(hidden, hidden), nn.ReLU(), nn.Dropout(0.1), nn.Linear(hidden, num_classes))

    def forward(self, x):
        x = F.elu(self.fc1(x))
        x = self.norm1(x)
        identity = x
        x = F.elu(self.fc2(x))
        x = self.norm2(x)
        x = x + identity
        x = F.elu(self.fc3(x))
        safety = torch.sigmoid(self.safety_head(x))
        anomaly = torch.sigmoid(self.anomaly_head(x))
        classes = self.class_head(x)
        return torch.cat([classes, safety, anomaly], dim=1)


def distillation_data(model, graphs, sample_count):
    features = []
    targets = []
    model.eval()
    with torch.no_grad():
        for graph in graphs:
            logits, safety, anomaly = model(graph.x, graph.edge_index)
            target = torch.cat([
                torch.softmax(logits, dim=1),
                torch.sigmoid(safety),
                torch.sigmoid(anomaly),
            ], dim=1)
            features.append(graph.x)
            targets.append(target)
            if sum(tensor.size(0) for tensor in features) >= sample_count:
                break
    return torch.cat(features)[:sample_count], torch.cat(targets)[:sample_count]


def train_mlp(features, targets, epochs, batch_size):
    split = int(features.size(0) * 0.9)
    train_dataset = TensorDataset(features[:split], targets[:split])
    val_x, val_y = features[split:], targets[split:]
    loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True)
    model = NodeMLP()
    optimizer = torch.optim.AdamW(model.parameters(), lr=0.001, weight_decay=0.001)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs)
    best_state = None
    best_mae = float("inf")
    for epoch in range(1, epochs + 1):
        model.train()
        for x, y in loader:
            optimizer.zero_grad(set_to_none=True)
            prediction = model(x)
            loss = F.mse_loss(prediction, y)
            loss.backward()
            optimizer.step()
        scheduler.step()
        model.eval()
        with torch.no_grad():
            mae = (model(val_x) - val_y).abs().mean().item()
        if mae < best_mae:
            best_mae = mae
            best_state = {key: value.detach().clone() for key, value in model.state_dict().items()}
        if epoch == 1 or epoch % 25 == 0:
            print(f"Distill epoch {epoch}/{epochs}  val_mae={mae:.6f}", flush=True)
    model.load_state_dict(best_state)
    model.eval()
    return model, best_mae, val_x, val_y


def export_model(model):
    import coremltools as ct
    traced = torch.jit.trace(model, torch.zeros(1, NUM_FEATURES))
    batch = ct.RangeDim(lower_bound=1, upper_bound=600, default=1)
    converted = ct.convert(
        traced,
        inputs=[ct.TensorType(name="node_features", shape=(batch, NUM_FEATURES))],
        outputs=[ct.TensorType(name="predictions")],
        minimum_deployment_target=ct.target.macOS13,
        convert_to="mlprogram",
    )
    converted.short_description = "X-MaC per-node classifier distilled from the filesystem GAT"
    converted.author = "X-MaC"
    converted.license = "MIT"
    if COREML_EXPORT_PATH.exists():
        backup_path = COREML_EXPORT_PATH.with_name("XMacGNN.previous.mlpackage")
        suffix = 1
        while backup_path.exists():
            backup_path = COREML_EXPORT_PATH.with_name(f"XMacGNN.previous-{suffix}.mlpackage")
            suffix += 1
        COREML_EXPORT_PATH.rename(backup_path)
    converted.save(str(COREML_EXPORT_PATH))
    return converted


def verify_coreml(mlmodel, model, features, targets, distill_mae):
    pytorch_output = model(features).detach().numpy()
    coreml_output = mlmodel.predict({"node_features": features.numpy()})["predictions"]
    coreml_mae = float(np.abs(coreml_output - pytorch_output).mean())
    teacher_mae = float(np.abs(coreml_output - targets.numpy()).mean())
    size_bytes = sum(path.stat().st_size for path in COREML_EXPORT_PATH.rglob("*") if path.is_file())
    results = {
        "distillation_mae": distill_mae,
        "coreml_pytorch_mae": coreml_mae,
        "coreml_teacher_mae": teacher_mae,
        "output_shape": list(coreml_output.shape),
        "size_bytes": size_bytes,
        "size_mb": size_bytes / 1024 / 1024,
    }
    (GNN_DIR / "coreml_verification.json").write_text(json.dumps(results, indent=2))
    print(json.dumps(results, indent=2))
    if distill_mae > 0.05 or teacher_mae > 0.05 or coreml_mae > 0.001 or size_bytes >= 5 * 1024 * 1024:
        raise SystemExit("CoreML acceptance criteria not met")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", type=Path, default=CHECKPOINT_PATH)
    parser.add_argument("--samples", type=int, default=10_000)
    parser.add_argument("--epochs", type=int, default=500)
    parser.add_argument("--batch-size", type=int, default=512)
    args = parser.parse_args()
    random.seed(42)
    torch.manual_seed(42)
    teacher, _ = load_model(args.checkpoint)
    # Training data uses PyG Data objects which require pickle.
    # TODO: migrate to safetensors to enable weights_only=True.
    graphs = torch.load(DATA_DIR / "train.pt", weights_only=False)
    features, targets = distillation_data(teacher, graphs, args.samples)
    permutation = torch.randperm(features.size(0))
    features, targets = features[permutation], targets[permutation]
    print(f"Distillation samples: {features.size(0)}")
    model, distill_mae, val_x, val_y = train_mlp(features, targets, args.epochs, args.batch_size)
    DISTILL_CHECKPOINT_PATH.parent.mkdir(parents=True, exist_ok=True)
    torch.save({"model_state_dict": model.state_dict(), "num_features": NUM_FEATURES}, DISTILL_CHECKPOINT_PATH)
    mlmodel = export_model(model)
    verify_coreml(mlmodel, model, val_x[:200], val_y[:200], distill_mae)


if __name__ == "__main__":
    main()
