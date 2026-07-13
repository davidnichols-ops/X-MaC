#!/usr/bin/env python3
"""Export the MemoryGAT model to CoreML via knowledge distillation.

The GNN requires graph structure (edge_index) at inference, which CoreML
doesn't support natively. We distill the GNN into a per-node MLP that only
needs node features — the graph structure is implicit in the learned weights.

Pipeline:
    1. Load trained MemoryGAT (teacher)
    2. Run teacher on training graphs to get soft targets
    3. Train a NodeMLP (student) to match teacher outputs
    4. Export NodeMLP to CoreML

Usage:
    python export_memory_coreml.py [--samples 20000] [--epochs 500]
"""

import argparse
import json
import os
import random
import sys
from pathlib import Path

os.environ.setdefault("KMP_DUPLICATE_LIB_OK", "TRUE")

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, TensorDataset

sys.path.insert(0, str(Path(__file__).resolve().parent))

from model.memory_gnn import MemoryGAT, NUM_ACTIONS, NUM_PRESSURE, MAX_FEATURE_DIM
from memory_data_generator import MEMORY_DATA_DIR

GNN_DIR = Path(__file__).resolve().parent
COREML_EXPORT_PATH = GNN_DIR / "XMacMemoryGNN.mlpackage"
CHECKPOINT_PATH = GNN_DIR / "model" / "memory_gnn.pt"
DISTILL_CHECKPOINT_PATH = GNN_DIR / "model" / "memory_node_mlp.pt"

# Output: action_logits (6) + risk (1) + growth (1) + pressure (3) = 11
OUTPUT_DIM = NUM_ACTIONS + 1 + 1 + NUM_PRESSURE


class MemoryNodeMLP(nn.Module):
    """Per-node MLP distilled from MemoryGAT.

    Input:  node features [N, 24]
    Output:  action_logits (6) + risk (1) + growth (1) + pressure (3) = 11 values

    The pressure prediction uses the node's features alone (no pooling),
    which works because the hardware/swap/compressor nodes encode system state.
    """

    def __init__(self, in_features=MAX_FEATURE_DIM, hidden=128, output_dim=OUTPUT_DIM):
        super().__init__()
        self.fc1 = nn.Linear(in_features, hidden)
        self.fc2 = nn.Linear(hidden, hidden)
        self.fc3 = nn.Linear(hidden, hidden)
        self.norm1 = nn.LayerNorm(hidden)
        self.norm2 = nn.LayerNorm(hidden)
        self.out_head = nn.Sequential(
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden, output_dim),
        )

    def forward(self, x):
        x = F.elu(self.fc1(x))
        x = self.norm1(x)
        identity = x
        x = F.elu(self.fc2(x))
        x = self.norm2(x)
        x = x + identity
        x = F.elu(self.fc3(x))
        return self.out_head(x)


def distillation_data(teacher, graphs, sample_count, device):
    """Run teacher GNN on graphs to collect features + soft targets."""
    teacher.eval()
    features = []
    targets = []
    total_collected = 0
    with torch.no_grad():
        for i, graph in enumerate(graphs):
            try:
                # Move to CPU for inference (avoid MPS issues)
                x = graph.x.to('cpu')
                edge_index = graph.edge_index.to('cpu')
                action_logits, risk_scores, growth_preds, pressure_logits = teacher(
                    x, edge_index, None
                )
                # Build target: softmax(action) + sigmoid(risk) + growth + softmax(pressure)
                # Pressure is per-graph; broadcast to all nodes
                pressure_soft = F.softmax(pressure_logits, dim=-1)  # [1, 3]
                num_nodes = x.size(0)
                pressure_per_node = pressure_soft.expand(num_nodes, -1)  # [N, 3]

                target = torch.cat([
                    F.softmax(action_logits, dim=-1),    # [N, 6]
                    risk_scores,                          # [N, 1]
                    growth_preds,                         # [N, 1]
                    pressure_per_node,                    # [N, 3]
                ], dim=1)  # [N, 11]

                features.append(x.cpu())
                targets.append(target.cpu())
                total_collected += x.size(0)
                if total_collected >= sample_count:
                    break
                if (i + 1) % 100 == 0:
                    print(f"  Processed {i+1} graphs, {total_collected} samples", flush=True)
            except Exception as e:
                print(f"  Skipping graph {i}: {e}", flush=True)
                continue

    return torch.cat(features)[:sample_count], torch.cat(targets)[:sample_count]


def train_mlp(features, targets, epochs, batch_size):
    """Train the student MLP to match teacher outputs."""
    split = int(features.size(0) * 0.9)
    train_dataset = TensorDataset(features[:split], targets[:split])
    val_x, val_y = features[split:], targets[split:]
    loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True)

    model = MemoryNodeMLP()
    optimizer = torch.optim.AdamW(model.parameters(), lr=0.001, weight_decay=0.001)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs)

    best_state = None
    best_mae = float("inf")

    for epoch in range(1, epochs + 1):
        model.train()
        for x, y in loader:
            optimizer.zero_grad(set_to_none=True)
            pred = model(x)
            loss = F.mse_loss(pred, y)
            loss.backward()
            optimizer.step()
        scheduler.step()

        model.eval()
        with torch.no_grad():
            mae = (model(val_x) - val_y).abs().mean().item()
        if mae < best_mae:
            best_mae = mae
            best_state = {k: v.detach().clone() for k, v in model.state_dict().items()}
        if epoch == 1 or epoch % 25 == 0:
            print(f"Distill epoch {epoch}/{epochs}  val_mae={mae:.6f}", flush=True)

    model.load_state_dict(best_state)
    model.eval()
    return model, best_mae, val_x, val_y


def export_model(model):
    """Export the MLP to CoreML."""
    import coremltools as ct
    traced = torch.jit.trace(model, torch.zeros(1, MAX_FEATURE_DIM))
    batch = ct.RangeDim(lower_bound=1, upper_bound=600, default=1)
    converted = ct.convert(
        traced,
        inputs=[ct.TensorType(name="node_features", shape=(batch, MAX_FEATURE_DIM))],
        outputs=[ct.TensorType(name="predictions")],
        minimum_deployment_target=ct.target.macOS13,
        convert_to="mlprogram",
    )
    converted.short_description = "X-MaC Memory Optimizer distilled from MemoryGAT"
    converted.author = "X-MaC"
    converted.license = "MIT"
    if COREML_EXPORT_PATH.exists():
        backup = COREML_EXPORT_PATH.with_name("XMacMemoryGNN.previous.mlpackage")
        suffix = 1
        while backup.exists():
            backup = COREML_EXPORT_PATH.with_name(f"XMacMemoryGNN.previous-{suffix}.mlpackage")
            suffix += 1
        COREML_EXPORT_PATH.rename(backup)
    converted.save(str(COREML_EXPORT_PATH))
    return converted


def verify_coreml(mlmodel, model, features, targets, distill_mae):
    """Verify CoreML output matches PyTorch."""
    pytorch_output = model(features).detach().numpy()
    coreml_output = mlmodel.predict({"node_features": features.numpy()})["predictions"]
    coreml_mae = float(np.abs(coreml_output - pytorch_output).mean())
    teacher_mae = float(np.abs(coreml_output - targets.numpy()).mean())
    size_bytes = sum(p.stat().st_size for p in COREML_EXPORT_PATH.rglob("*") if p.is_file())
    results = {
        "distillation_mae": distill_mae,
        "coreml_pytorch_mae": coreml_mae,
        "coreml_teacher_mae": teacher_mae,
        "output_shape": list(coreml_output.shape),
        "output_desc": "action_logits(6) + risk(1) + growth(1) + pressure(3) = 11",
        "size_bytes": size_bytes,
        "size_mb": size_bytes / 1024 / 1024,
    }
    (GNN_DIR / "memory_coreml_verification.json").write_text(json.dumps(results, indent=2))
    print(json.dumps(results, indent=2))
    if coreml_mae > 0.001:
        raise SystemExit("CoreML acceptance criteria not met")


def main():
    parser = argparse.ArgumentParser(description="Export MemoryGAT to CoreML")
    parser.add_argument("--checkpoint", type=Path, default=CHECKPOINT_PATH)
    parser.add_argument("--samples", type=int, default=20_000)
    parser.add_argument("--epochs", type=int, default=500)
    parser.add_argument("--batch-size", type=int, default=512)
    parser.add_argument("--no-mps", action="store_true", help="Disable MPS")
    args = parser.parse_args()

    random.seed(42)
    torch.manual_seed(42)

    device = torch.device('cpu')
    if torch.backends.mps.is_available() and not args.no_mps:
        device = torch.device('mps')

    # Load teacher (always on CPU for stability)
    teacher = MemoryGAT()
    teacher.load_state_dict(torch.load(args.checkpoint, weights_only=True, map_location='cpu'))
    teacher = teacher.to('cpu')
    teacher.eval()
    print(f"Teacher loaded from {args.checkpoint}", flush=True)

    # Load graphs
    train_graphs = torch.load(MEMORY_DATA_DIR / "train.pt", weights_only=False)
    print(f"Loaded {len(train_graphs)} training graphs", flush=True)

    # Distill
    features, targets = distillation_data(teacher, train_graphs, args.samples, device)
    perm = torch.randperm(features.size(0))
    features, targets = features[perm], targets[perm]
    print(f"Distillation samples: {features.size(0)}", flush=True)

    model, distill_mae, val_x, val_y = train_mlp(features, targets, args.epochs, args.batch_size)
    print(f"Best distillation MAE: {distill_mae:.6f}", flush=True)

    # Save student
    DISTILL_CHECKPOINT_PATH.parent.mkdir(parents=True, exist_ok=True)
    torch.save({
        "model_state_dict": model.state_dict(),
        "num_features": MAX_FEATURE_DIM,
        "output_dim": OUTPUT_DIM,
    }, DISTILL_CHECKPOINT_PATH)
    print(f"Student saved to {DISTILL_CHECKPOINT_PATH}", flush=True)

    # Export to CoreML
    mlmodel = export_model(model)
    print(f"CoreML saved to {COREML_EXPORT_PATH}", flush=True)

    # Verify
    verify_coreml(mlmodel, model, val_x[:200], val_y[:200], distill_mae)


if __name__ == "__main__":
    main()
