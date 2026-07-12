#!/usr/bin/env python3
"""
X-MaC GNN Training Script
=========================
Generates synthetic file-system graphs, trains the GAT model, and exports
to CoreML (.mlpackage) for on-device inference.

The synthetic data generator creates realistic file-system graphs with:
- Cache directories (safe to clean, safety=0.9)
- Build artifacts (safe to clean, safety=0.85)
- Log files (safe to clean, safety=0.8)
- Config files (NOT safe, safety=0.1)
- Source code (NOT safe, safety=0.05)
- System files (NOT safe, safety=0.0)
- Large media files (review, safety=0.5)
- Trash (very safe, safety=0.95)

Features (9-dim, matching Rust extractor):
  [log_size, depth_norm, is_dir, is_file, is_symlink, is_hidden,
   age_days_norm, access_age_days_norm, has_extension]
"""

import os
import sys
import json
import random
import math
import time
from pathlib import Path

# Must set before importing torch
os.environ["KMP_DUPLICATE_LIB_OK"] = "TRUE"

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch_geometric.data import Data, DataLoader
from torch_geometric.nn import GATConv, global_mean_pool

# ─── Config ───────────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
MODEL_DIR = PROJECT_ROOT / "gnn"
LABEL_MAP_PATH = MODEL_DIR / "label_map.json"
COREML_EXPORT_PATH = MODEL_DIR / "XMacGNN.mlpackage"

NUM_FEATURES = 9
HIDDEN_DIM = 64
NUM_CLASSES = 27
NUM_HEADS = 4
EPOCHS = 200
BATCH_SIZE = 32
LR = 0.001
NUM_SYNTHETIC_GRAPHS = 500
NODES_PER_GRAPH = 80

# ─── Label map ────────────────────────────────────────────────────────────

with open(LABEL_MAP_PATH) as f:
    LABEL_MAP = json.load(f)
LABEL_TO_IDX = {v: k for k, v in LABEL_MAP.items()}
IDX_TO_LABEL = {v: k for k, v in LABEL_MAP.items()}

# ─── Synthetic data generator ─────────────────────────────────────────────

# Category → (label_idx, safety_score, anomaly_score, typical_size_range)
CATEGORIES = [
    # (label_name, safety, anomaly, size_min, size_max, depth_max, is_dir, ext)
    ("cache_dir",      0.92, 0.15, 1e6, 5e9,   4, True,  None),
    ("cache_file",     0.90, 0.10, 1e3, 5e8,   5, False, "cache"),
    ("build_output",   0.85, 0.20, 1e4, 1e9,   5, False, "o"),
    ("cargo_target",   0.88, 0.15, 1e4, 5e8,   5, False, "rlib"),
    ("python_cache",   0.90, 0.10, 1e2, 5e7,   5, False, "pyc"),
    ("log_file",       0.80, 0.30, 1e2, 5e7,   4, False, "log"),
    ("log_dir",        0.82, 0.25, 1e5, 5e8,   3, True,  None),
    ("trash",          0.95, 0.05, 1e3, 1e9,   2, False, None),
    ("temp_file",      0.85, 0.20, 1e2, 5e8,   4, False, "tmp"),
    ("disk_image",     0.60, 0.40, 1e7, 5e9,   3, False, "dmg"),
    ("archive",        0.55, 0.35, 1e6, 5e9,   4, False, "zip"),
    ("backup_dir",     0.50, 0.45, 1e8, 5e10,  3, True,  None),
    ("language_file",  0.70, 0.20, 1e3, 5e6,   5, False, "lproj"),
    ("package_manager_cache", 0.88, 0.15, 1e4, 5e8, 4, False, "gz"),
    ("config_file",    0.10, 0.50, 1e2, 5e5,   4, False, "json"),
    ("source_code",    0.05, 0.60, 1e2, 5e6,   5, False, "rs"),
    ("executable",     0.15, 0.55, 1e4, 5e7,   4, False, None),
    ("library_file",   0.20, 0.50, 1e5, 5e8,   4, False, "dylib"),
    ("library_dir",    0.15, 0.40, 1e6, 5e9,   3, True,  None),
    ("document",       0.25, 0.35, 1e3, 5e7,   4, False, "pdf"),
    ("image",          0.40, 0.30, 1e4, 5e8,   4, False, "png"),
    ("video",          0.45, 0.35, 1e6, 5e9,   4, False, "mp4"),
    ("audio",          0.35, 0.30, 1e4, 5e8,   4, False, "mp3"),
    ("git_dir",        0.08, 0.55, 1e3, 5e8,   3, True,  None),
    ("app_bundle",     0.12, 0.45, 1e7, 5e9,   2, True,  None),
    ("directory",      0.30, 0.25, 0,   0,     3, True,  None),
    ("root",           0.00, 0.50, 0,   0,     0, True,  None),
]


def make_node_features(size_bytes, depth, is_dir, is_file, is_symlink,
                       is_hidden, age_days, access_days, has_ext, max_depth=6):
    return [
        math.log(size_bytes + 1) / 30.0 if size_bytes > 0 else 0.0,
        min(depth / max_depth, 1.0),
        1.0 if is_dir else 0.0,
        1.0 if is_file else 0.0,
        1.0 if is_symlink else 0.0,
        1.0 if is_hidden else 0.0,
        min(age_days / 365.0, 1.0),
        min(access_days / 365.0, 1.0),
        1.0 if has_ext else 0.0,
    ]


def generate_graph(num_nodes=NODES_PER_GRAPH):
    """Generate a synthetic file-system graph with realistic labels."""
    nodes = []
    edges_src = []
    edges_dst = []

    # Root node
    nodes.append({
        "features": make_node_features(0, 0, True, False, False, False, 0, 0, False),
        "label": LABEL_MAP["root"],  # 23
        "safety": 0.0,
        "anomaly": 0.5,
    })

    for i in range(1, num_nodes):
        cat = random.choice(CATEGORIES)
        label_name, safety, anomaly, smin, smax, dmax, is_dir, ext = cat

        depth = random.randint(1, max(dmax, 1))
        size = random.uniform(smin, smax) if smax > 0 else 0
        is_hidden = random.random() < 0.3
        age_days = random.randint(0, 365 * 3)
        access_days = random.randint(0, age_days + 1) if age_days > 0 else 0
        has_ext = ext is not None
        is_symlink = random.random() < 0.02

        features = make_node_features(
            size, depth, is_dir, not is_dir, is_symlink,
            is_hidden, age_days, access_days, has_ext
        )

        # Add some noise to safety/anomaly for realism
        safety_noisy = max(0.0, min(1.0, safety + random.gauss(0, 0.05)))
        anomaly_noisy = max(0.0, min(1.0, anomaly + random.gauss(0, 0.05)))

        label_idx = LABEL_MAP.get(label_name, 13)  # default: file

        nodes.append({
            "features": features,
            "label": label_idx,  # already an int
            "safety": safety_noisy,
            "anomaly": anomaly_noisy,
        })

        # Connect to a random parent (earlier node)
        parent = random.randint(0, i - 1)
        edges_src.append(parent)
        edges_dst.append(i)

    # Add some same-directory edges (siblings)
    for _ in range(num_nodes // 4):
        a = random.randint(1, num_nodes - 1)
        b = random.randint(1, num_nodes - 1)
        if a != b:
            edges_src.append(a)
            edges_dst.append(b)

    x = torch.tensor([n["features"] for n in nodes], dtype=torch.float32)
    edge_index = torch.tensor([edges_src, edges_dst], dtype=torch.long)
    labels = torch.tensor([n["label"] for n in nodes], dtype=torch.long)
    safety = torch.tensor([n["safety"] for n in nodes], dtype=torch.float32)
    anomaly = torch.tensor([n["anomaly"] for n in nodes], dtype=torch.float32)

    return Data(x=x, edge_index=edge_index, y=labels,
                safety=safety, anomaly=anomaly)


def generate_dataset(n=NUM_SYNTHETIC_GRAPHS):
    """Generate a dataset of synthetic file-system graphs."""
    graphs = []
    for _ in range(n):
        n_nodes = random.randint(30, NODES_PER_GRAPH)
        graphs.append(generate_graph(n_nodes))
    return graphs


# ─── GAT Model (matches gnn/model/gnn.py) ─────────────────────────────────

class GATModel(nn.Module):
    def __init__(self, num_features=NUM_FEATURES, hidden_dim=HIDDEN_DIM,
                 num_classes=NUM_CLASSES, num_heads=NUM_HEADS):
        super().__init__()
        self.conv1 = GATConv(num_features, hidden_dim, heads=num_heads, dropout=0.1)
        self.conv2 = GATConv(hidden_dim * num_heads, hidden_dim, heads=1, dropout=0.1)

        self.safety_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden_dim // 2, 1)
        )
        self.anomaly_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden_dim // 2, 1)
        )
        self.class_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Linear(hidden_dim // 2, num_classes)
        )

    def forward(self, x, edge_index, batch=None):
        x = F.elu(self.conv1(x, edge_index))
        x = F.dropout(x, p=0.1, training=self.training)
        x = F.elu(self.conv2(x, edge_index))

        safety = self.safety_head(x)
        anomaly = self.anomaly_head(x)
        logits = self.class_head(x)

        return logits, safety, anomaly


# ─── Training loop ────────────────────────────────────────────────────────

def train():
    print("=" * 60)
    print("X-MaC GNN Training")
    print("=" * 60)

    # Generate dataset
    print(f"\nGenerating {NUM_SYNTHETIC_GRAPHS} synthetic graphs...")
    graphs = generate_dataset(NUM_SYNTHETIC_GRAPHS)
    print(f"  Total nodes: {sum(g.num_nodes for g in graphs)}")
    print(f"  Total edges: {sum(g.num_edges for g in graphs)}")

    # Split 80/20
    split = int(len(graphs) * 0.8)
    train_graphs = graphs[:split]
    val_graphs = graphs[split:]
    print(f"  Train: {len(train_graphs)}  Val: {len(val_graphs)}")

    train_loader = DataLoader(train_graphs, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_graphs, batch_size=BATCH_SIZE, shuffle=False)

    # Model
    device = torch.device("cpu")
    model = GATModel().to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=LR, weight_decay=1e-4)

    # Loss weights
    W_CLASS = 1.0
    W_SAFETY = 2.0
    W_ANOMALY = 1.0

    best_val_loss = float("inf")
    best_state = None

    print(f"\nTraining for {EPOCHS} epochs...")
    print(f"  Features: {NUM_FEATURES}  Hidden: {HIDDEN_DIM}  Classes: {NUM_CLASSES}")
    print(f"  LR: {LR}  Batch: {BATCH_SIZE}  Heads: {NUM_HEADS}")
    print()

    for epoch in range(1, EPOCHS + 1):
        # Train
        model.train()
        total_loss = 0
        for batch in train_loader:
            batch = batch.to(device)
            optimizer.zero_grad()
            logits, safety, anomaly = model(batch.x, batch.edge_index)

            loss_class = F.cross_entropy(logits, batch.y)
            loss_safety = F.mse_loss(safety.squeeze(-1), batch.safety)
            loss_anomaly = F.mse_loss(anomaly.squeeze(-1), batch.anomaly)

            loss = W_CLASS * loss_class + W_SAFETY * loss_safety + W_ANOMALY * loss_anomaly
            loss.backward()
            optimizer.step()
            total_loss += loss.item() * batch.num_graphs

        train_loss = total_loss / len(train_graphs)

        # Validate
        model.eval()
        val_loss = 0
        val_acc = 0
        val_safety_mae = 0
        with torch.no_grad():
            for batch in val_loader:
                batch = batch.to(device)
                logits, safety, anomaly = model(batch.x, batch.edge_index)

                loss_class = F.cross_entropy(logits, batch.y)
                loss_safety = F.mse_loss(safety.squeeze(-1), batch.safety)
                loss_anomaly = F.mse_loss(anomaly.squeeze(-1), batch.anomaly)
                loss = W_CLASS * loss_class + W_SAFETY * loss_safety + W_ANOMALY * loss_anomaly
                val_loss += loss.item() * batch.num_graphs

                pred = logits.argmax(dim=1)
                val_acc += (pred == batch.y).float().mean().item() * batch.num_graphs
                val_safety_mae += (safety.squeeze(-1) - batch.safety).abs().mean().item() * batch.num_graphs

        val_loss /= len(val_graphs)
        val_acc /= len(val_graphs)
        val_safety_mae /= len(val_graphs)

        if val_loss < best_val_loss:
            best_val_loss = val_loss
            best_state = {k: v.clone() for k, v in model.state_dict().items()}

        if epoch % 10 == 0 or epoch == 1:
            print(f"  Epoch {epoch:3d}/{EPOCHS}  "
                  f"train_loss={train_loss:.4f}  val_loss={val_loss:.4f}  "
                  f"val_acc={val_acc:.3f}  safety_mae={val_safety_mae:.4f}")

    print(f"\nBest validation loss: {best_val_loss:.4f}")

    # Load best model
    if best_state:
        model.load_state_dict(best_state)

    # Save PyTorch checkpoint
    ckpt_path = MODEL_DIR / "model" / "xmac_gnn.pt"
    ckpt_path.parent.mkdir(parents=True, exist_ok=True)
    torch.save({
        "model_state_dict": model.state_dict(),
        "num_features": NUM_FEATURES,
        "num_classes": NUM_CLASSES,
        "hidden_dim": HIDDEN_DIM,
        "num_heads": NUM_HEADS,
    }, ckpt_path)
    print(f"Saved PyTorch checkpoint: {ckpt_path}")

    # Export to CoreML
    export_coreml(model)

    # Quick sanity check
    print("\n=== Sanity check ===")
    test_graph = generate_graph(20)
    model.eval()
    with torch.no_grad():
        logits, safety, anomaly = model(test_graph.x, test_graph.edge_index)
        safety_scores = torch.sigmoid(safety.squeeze(-1))
        print(f"  Test graph: {test_graph.num_nodes} nodes, {test_graph.num_edges} edges")
        print(f"  Safety scores: min={safety_scores.min():.3f} max={safety_scores.max():.3f} mean={safety_scores.mean():.3f}")
        print(f"  Label accuracy: {(logits.argmax(1) == test_graph.y).float().mean():.3f}")

    print("\nDone!")


def export_coreml(model):
    """Export a simplified MLP-only model to CoreML.

    CoreML doesn't support graph neural network operations (scatter_reduce,
    edge_index-based message passing). We export a per-node MLP that takes
    the 9-feature vector and outputs [logits(27), safety, anomaly] = 29 values.

    The graph structure is handled in Swift/Rust — the CoreML model just
    does the per-node scoring on the already-extracted features.
    """
    try:
        import coremltools as ct
    except ImportError:
        print("  coremltools not available — skipping CoreML export")
        return

    print("\nExporting to CoreML (per-node MLP)...")

    model.eval()

    # Build a standalone MLP that replicates the heads after a frozen
    # 2-layer feature transform (we just use the heads directly since
    # the GAT conv output can't be replicated without edge_index).
    # In practice, the Swift side feeds pre-computed features and the
    # CoreML model acts as a safety classifier.
    class NodeMLP(nn.Module):
        """Per-node safety/anomaly/classification MLP for CoreML."""
        def __init__(self, in_features=NUM_FEATURES, hidden=HIDDEN_DIM, num_classes=NUM_CLASSES):
            super().__init__()
            self.fc1 = nn.Linear(in_features, hidden)
            self.fc2 = nn.Linear(hidden, hidden)
            self.safety_head = nn.Sequential(
                nn.Linear(hidden, hidden // 2),
                nn.ReLU(),
                nn.Linear(hidden // 2, 1)
            )
            self.anomaly_head = nn.Sequential(
                nn.Linear(hidden, hidden // 2),
                nn.ReLU(),
                nn.Linear(hidden // 2, 1)
            )
            self.class_head = nn.Sequential(
                nn.Linear(hidden, hidden // 2),
                nn.ReLU(),
                nn.Linear(hidden // 2, num_classes)
            )

        def forward(self, x):
            x = F.elu(self.fc1(x))
            x = F.elu(self.fc2(x))
            safety = torch.sigmoid(self.safety_head(x))
            anomaly = torch.sigmoid(self.anomaly_head(x))
            logits = self.class_head(x)
            return torch.cat([logits, safety, anomaly], dim=1)

    # Train the MLP to match the GNN's per-node outputs
    print("  Training distillation MLP to match GNN outputs...")
    mlp = NodeMLP()
    mlp_optimizer = torch.optim.Adam(mlp.parameters(), lr=0.001)

    # Generate distillation data: run GNN on synthetic graphs, collect (features → outputs)
    print("  Generating distillation dataset...")
    distill_x = []
    distill_y = []
    model.eval()
    with torch.no_grad():
        for _ in range(50):
            g = generate_graph(random.randint(40, 80))
            logits, safety, anomaly = model(g.x, g.edge_index)
            safety_sig = torch.sigmoid(safety.squeeze(-1))
            anomaly_sig = torch.sigmoid(anomaly.squeeze(-1))
            target = torch.cat([logits, safety_sig.unsqueeze(-1), anomaly_sig.unsqueeze(-1)], dim=1)
            distill_x.append(g.x)
            distill_y.append(target)

    distill_x = torch.cat(distill_x)
    distill_y = torch.cat(distill_y)
    print(f"  Distillation data: {distill_x.shape[0]} samples")

    # Train MLP
    for epoch in range(100):
        mlp_optimizer.zero_grad()
        pred = mlp(distill_x)
        loss = F.mse_loss(pred, distill_y)
        loss.backward()
        mlp_optimizer.step()
        if (epoch + 1) % 20 == 0:
            print(f"    Distill epoch {epoch+1}/100  loss={loss.item():.4f}")

    mlp.eval()

    # Verify
    with torch.no_grad():
        test_out = mlp(distill_x[:5])
        gnn_out = distill_y[:5]
        mae = (test_out - gnn_out).abs().mean().item()
        print(f"  Distillation MAE: {mae:.4f}")

    # Export to CoreML
    dummy = torch.randn(1, NUM_FEATURES)
    traced = torch.jit.trace(mlp, dummy)

    mlmodel = ct.convert(
        traced,
        inputs=[ct.TensorType(name="node_features", shape=[1, NUM_FEATURES])],
        outputs=[ct.TensorType(name="predictions")],
        minimum_deployment_target=ct.target.iOS15,
    )

    mlmodel.short_description = "X-MaC GNN: per-node safety + anomaly + classification (distilled from GAT)"
    mlmodel.author = "X-MaC"
    mlmodel.license = "MIT"

    # Remove old model
    if COREML_EXPORT_PATH.exists():
        import shutil
        shutil.rmtree(COREML_EXPORT_PATH)

    mlmodel.save(str(COREML_EXPORT_PATH))
    size_kb = sum(f.stat().st_size for f in COREML_EXPORT_PATH.rglob("*") if f.is_file()) // 1024
    print(f"  CoreML model saved: {COREML_EXPORT_PATH}")
    print(f"  Size: {size_kb} KB")


if __name__ == "__main__":
    random.seed(42)
    torch.manual_seed(42)
    train()
