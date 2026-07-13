#!/usr/bin/env python3
"""Training script for the MemoryGAT model.

Trains a Graph Attention Network to predict:
- System pressure trajectory (normal/warn/critical in 60s)
- Per-process action recommendation (6 classes)
- Per-process risk score (0.0-1.0)
- Per-process growth prediction (normalized RSS in 60s)

Usage:
    python train_memory_gnn.py [--epochs 500] [--batch-size 64]
"""

import argparse
import json
import math
import os
import sys
from collections import Counter
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch_geometric.loader import DataLoader

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from model.memory_gnn import MemoryGAT, NUM_ACTIONS, NUM_PRESSURE
from memory_data_generator import MEMORY_DATA_DIR

# ═══════════════════════════════════════════════════════════════════════
#  Configuration
# ═══════════════════════════════════════════════════════════════════════

GNN_DIR = Path(__file__).resolve().parent
MODEL_DIR = GNN_DIR / "model"
CHECKPOINT_PATH = MODEL_DIR / "memory_gnn.pt"
METRICS_PATH = MODEL_DIR / "memory_training_metrics.json"
HISTORY_PATH = MODEL_DIR / "training_history.jsonl"

# Training defaults
DEFAULT_EPOCHS = 500
DEFAULT_BATCH_SIZE = 64
DEFAULT_LR = 0.001
DEFAULT_WEIGHT_DECAY = 0.001
DEFAULT_PATIENCE = 100
MIN_PER_CLASS_ACCURACY = 0.80
MAX_RISK_MAE = 0.10
MAX_GROWTH_MAE = 0.05
MAX_PRESSURE_ACCURACY = 0.90


# ═══════════════════════════════════════════════════════════════════════
#  Loss function
# ═══════════════════════════════════════════════════════════════════════

class MemoryLoss(nn.Module):
    """Weighted multi-task loss for memory optimization."""

    def __init__(self, action_weights=None, pressure_weights=None):
        super().__init__()
        # Class weights for action (inverse frequency)
        self.action_weights = action_weights
        self.pressure_weights = pressure_weights

    def forward(self, outputs, data):
        action_logits, risk_scores, growth_preds, pressure_logits = outputs

        # Mask: only compute loss on process nodes (node_type == 0)
        if hasattr(data, 'node_type'):
            mask = data.node_type == 0
        else:
            mask = torch.ones(action_logits.size(0), dtype=torch.bool, device=action_logits.device)

        # Action classification loss (weighted)
        if mask.any():
            action_loss = F.cross_entropy(
                action_logits[mask],
                data.action_labels[mask],
                weight=self.action_weights.to(action_logits.device) if self.action_weights is not None else None,
            )
            risk_loss = F.mse_loss(risk_scores[mask].squeeze(-1), data.risk_labels[mask])
            growth_loss = F.mse_loss(growth_preds[mask].squeeze(-1), data.growth_labels[mask])
        else:
            action_loss = torch.tensor(0.0, device=action_logits.device)
            risk_loss = torch.tensor(0.0, device=action_logits.device)
            growth_loss = torch.tensor(0.0, device=action_logits.device)

        # Pressure classification loss (system-level, one per graph)
        pressure_loss = F.cross_entropy(
            pressure_logits,
            data.pressure_label,
            weight=self.pressure_weights.to(pressure_logits.device) if self.pressure_weights is not None else None,
        )

        # Weighted total
        total = 3.0 * action_loss + risk_loss + 0.5 * growth_loss + 2.0 * pressure_loss

        return total, {
            'action': action_loss.item(),
            'risk': risk_loss.item(),
            'growth': growth_loss.item(),
            'pressure': pressure_loss.item(),
            'total': total.item(),
        }


# ═══════════════════════════════════════════════════════════════════════
#  Training loop
# ═══════════════════════════════════════════════════════════════════════

def compute_class_weights(dataset, num_classes, label_key='action_labels', temper=0.5):
    """Compute tempered inverse-frequency class weights.

    Uses (1/freq)^temper instead of pure 1/freq to avoid extreme weights.
    temper=0.5 (sqrt) provides moderate rebalancing without ignoring common classes.
    """
    counts = Counter()
    for data in dataset:
        labels = getattr(data, label_key)
        if label_key == 'pressure_label':
            counts[labels.item()] += 1
        else:
            # Only count process nodes
            if hasattr(data, 'node_type'):
                mask = data.node_type == 0
                for l in labels[mask].tolist():
                    counts[l] += 1
            else:
                for l in labels.tolist():
                    counts[l] += 1

    total = sum(counts.values())
    weights = torch.ones(num_classes)
    for cls in range(num_classes):
        if counts.get(cls, 0) > 0:
            freq = counts[cls] / total
            weights[cls] = (1.0 / (num_classes * freq)) ** temper
        else:
            weights[cls] = 1.0

    # Normalize so mean = 1.0
    weights = weights / weights.mean()
    return weights


def evaluate(model, loader, device, loss_fn):
    """Evaluate the model on a data loader."""
    model.eval()
    total_loss = 0
    total_action_correct = 0
    total_action_total = 0
    total_risk_error = 0
    total_growth_error = 0
    total_pressure_correct = 0
    total_pressure_total = 0
    per_class_correct = Counter()
    per_class_total = Counter()
    loss_components = {'action': 0, 'risk': 0, 'growth': 0, 'pressure': 0, 'total': 0}
    num_batches = 0

    with torch.no_grad():
        for data in loader:
            data = data.to(device)
            outputs = model(data.x, data.edge_index, getattr(data, 'batch', None))
            loss, components = loss_fn(outputs, data)
            total_loss += loss.item()
            for k in loss_components:
                loss_components[k] += components[k]

            action_logits, risk_scores, growth_preds, pressure_logits = outputs

            # Action accuracy (process nodes only)
            if hasattr(data, 'node_type'):
                mask = data.node_type == 0
            else:
                mask = torch.ones(action_logits.size(0), dtype=torch.bool, device=device)

            preds = action_logits[mask].argmax(dim=-1)
            labels = data.action_labels[mask]
            total_action_correct += (preds == labels).sum().item()
            total_action_total += labels.size(0)

            for l, p in zip(labels.tolist(), preds.tolist()):
                per_class_total[l] += 1
                if l == p:
                    per_class_correct[l] += 1

            # Risk MAE
            total_risk_error += (risk_scores[mask].squeeze(-1) - data.risk_labels[mask]).abs().sum().item()

            # Growth MAE
            total_growth_error += (growth_preds[mask].squeeze(-1) - data.growth_labels[mask]).abs().sum().item()

            # Pressure accuracy
            pressure_preds = pressure_logits.argmax(dim=-1)
            total_pressure_correct += (pressure_preds == data.pressure_label).sum().item()
            total_pressure_total += data.pressure_label.size(0)

            num_batches += 1

    n = max(total_action_total, 1)
    metrics = {
        'loss': total_loss / max(num_batches, 1),
        'action_accuracy': total_action_correct / n,
        'risk_mae': total_risk_error / n,
        'growth_mae': total_growth_error / n,
        'pressure_accuracy': total_pressure_correct / max(total_pressure_total, 1),
        'per_class_accuracy': {
            cls: per_class_correct.get(cls, 0) / max(per_class_total.get(cls, 1), 1)
            for cls in range(NUM_ACTIONS)
        },
    }
    for k in loss_components:
        loss_components[k] /= max(num_batches, 1)
    metrics['loss_components'] = loss_components

    return metrics


def train(args):
    """Main training loop."""
    device = torch.device('cpu')
    if torch.cuda.is_available():
        device = torch.device('cuda')
    elif torch.backends.mps.is_available() and not args.no_mps:
        device = torch.device('mps')
    print(f"Device: {device}", flush=True)

    # Load data
    train_path = MEMORY_DATA_DIR / "train.pt"
    val_path = MEMORY_DATA_DIR / "val.pt"
    test_path = MEMORY_DATA_DIR / "test.pt"

    if not train_path.exists():
        print(f"Error: Training data not found at {train_path}")
        print("Run: python memory_data_generator.py --num-graphs 10000")
        sys.exit(1)

    train_data = torch.load(train_path, weights_only=False)
    val_data = torch.load(val_path, weights_only=False)
    test_data = torch.load(test_path, weights_only=False)

    print(f"Train: {len(train_data)} graphs")
    print(f"Val:   {len(val_data)} graphs")
    print(f"Test:  {len(test_data)} graphs")

    # Data loaders
    train_loader = DataLoader(train_data, batch_size=args.batch_size, shuffle=True)
    val_loader = DataLoader(val_data, batch_size=args.batch_size, shuffle=False)
    test_loader = DataLoader(test_data, batch_size=args.batch_size, shuffle=False)

    # Compute class weights
    action_weights = compute_class_weights(train_data, NUM_ACTIONS, 'action_labels')
    pressure_weights = compute_class_weights(train_data, NUM_PRESSURE, 'pressure_label')
    print(f"Action weights: {action_weights.tolist()}")
    print(f"Pressure weights: {pressure_weights.tolist()}")

    # Model
    model = MemoryGAT(
        hidden_dim=args.hidden_dim,
        num_heads=args.num_heads,
        num_layers=args.num_layers,
        dropout=args.dropout,
    ).to(device)

    num_params = sum(p.numel() for p in model.parameters())
    print(f"Model parameters: {num_params:,}")

    # Optimizer and scheduler
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=args.weight_decay)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingWarmRestarts(optimizer, T_0=50, T_mult=2)

    # Loss
    loss_fn = MemoryLoss(action_weights=action_weights, pressure_weights=pressure_weights)

    # Training loop
    best_val_accuracy = 0.0
    patience_counter = 0
    history = []

    # Clear jsonl log
    with open(HISTORY_PATH, 'w') as f:
        pass

    print(f"\nTraining for {args.epochs} epochs (patience={args.patience})...", flush=True)
    print(f"{'Epoch':>5} {'Train Loss':>10} {'Val Loss':>10} {'Action Acc':>10} {'Pressure':>10} {'Risk MAE':>10} {'Growth':>10}", flush=True)

    for epoch in range(1, args.epochs + 1):
        # Train
        model.train()
        train_loss = 0
        train_batches = 0
        for data in train_loader:
            data = data.to(device)
            optimizer.zero_grad()
            outputs = model(data.x, data.edge_index, getattr(data, 'batch', None))
            loss, _ = loss_fn(outputs, data)
            loss.backward()
            optimizer.step()
            train_loss += loss.item()
            train_batches += 1

        scheduler.step()

        # Validate
        val_metrics = evaluate(model, val_loader, device, loss_fn)

        # Log to console
        avg_train_loss = train_loss / max(train_batches, 1)
        print(f"{epoch:5d} {avg_train_loss:10.4f} {val_metrics['loss']:10.4f} "
              f"{val_metrics['action_accuracy']:10.4f} {val_metrics['pressure_accuracy']:10.4f} "
              f"{val_metrics['risk_mae']:10.4f} {val_metrics['growth_mae']:10.4f}", flush=True)

        # Log to jsonl
        record = {
            'epoch': epoch,
            'train_loss': avg_train_loss,
            'val_loss': val_metrics['loss'],
            'val_action_accuracy': val_metrics['action_accuracy'],
            'val_pressure_accuracy': val_metrics['pressure_accuracy'],
            'val_risk_mae': val_metrics['risk_mae'],
            'val_growth_mae': val_metrics['growth_mae'],
            'per_class_accuracy': val_metrics['per_class_accuracy'],
        }
        history.append(record)
        with open(HISTORY_PATH, 'a') as f:
            f.write(json.dumps(record) + '\n')

        # Early stopping
        current_acc = val_metrics['action_accuracy']
        if current_acc > best_val_accuracy:
            best_val_accuracy = current_acc
            patience_counter = 0
            # Save best model
            torch.save(model.state_dict(), CHECKPOINT_PATH)
        else:
            patience_counter += 1
            # Periodic checkpoint every 50 epochs even if not best
            if epoch % 50 == 0:
                torch.save(model.state_dict(), MODEL_DIR / f"memory_gnn_epoch{epoch}.pt")
            if patience_counter >= args.patience:
                print(f"\nEarly stopping at epoch {epoch} (patience={args.patience})", flush=True)
                break

    # Load best model and evaluate on test set
    print(f"\n{'='*60}", flush=True)
    print("Loading best model and evaluating on test set...", flush=True)
    model.load_state_dict(torch.load(CHECKPOINT_PATH, weights_only=True))
    test_metrics = evaluate(model, test_loader, device, loss_fn)

    print(f"\nTest Results:", flush=True)
    print(f"  Action Accuracy:    {test_metrics['action_accuracy']:.4f}", flush=True)
    print(f"  Pressure Accuracy:  {test_metrics['pressure_accuracy']:.4f}", flush=True)
    print(f"  Risk MAE:           {test_metrics['risk_mae']:.4f}", flush=True)
    print(f"  Growth MAE:         {test_metrics['growth_mae']:.4f}", flush=True)
    print(f"\nPer-Class Action Accuracy:", flush=True)
    action_names = ['no_action', 'pressure_relief', 'suggest_purge', 'deprioritize', 'suspend', 'terminate']
    for cls, name in enumerate(action_names):
        acc = test_metrics['per_class_accuracy'].get(cls, 0)
        print(f"  {name:20s} {acc:.4f}", flush=True)

    # Check acceptance criteria
    print(f"\n{'='*60}", flush=True)
    print("Acceptance Criteria:", flush=True)
    checks = [
        ("Action accuracy >= 0.85", test_metrics['action_accuracy'] >= 0.85),
        ("Pressure accuracy >= 0.90", test_metrics['pressure_accuracy'] >= 0.90),
        ("Risk MAE <= 0.10", test_metrics['risk_mae'] <= MAX_RISK_MAE),
        ("Growth MAE <= 0.05", test_metrics['growth_mae'] <= MAX_GROWTH_MAE),
        ("Min per-class accuracy >= 0.80",
         min(test_metrics['per_class_accuracy'].values()) >= MIN_PER_CLASS_ACCURACY),
    ]
    all_pass = True
    for desc, passed in checks:
        status = "PASS" if passed else "FAIL"
        print(f"  [{status}] {desc}", flush=True)
        if not passed:
            all_pass = False

    # Save metrics
    metrics = {
        'test_action_accuracy': test_metrics['action_accuracy'],
        'test_pressure_accuracy': test_metrics['pressure_accuracy'],
        'test_risk_mae': test_metrics['risk_mae'],
        'test_growth_mae': test_metrics['growth_mae'],
        'test_per_class_accuracy': test_metrics['per_class_accuracy'],
        'best_val_accuracy': best_val_accuracy,
        'num_parameters': num_params,
        'epochs_trained': len(history),
        'all_criteria_passed': all_pass,
    }
    with open(METRICS_PATH, 'w') as f:
        json.dump(metrics, f, indent=2)

    print(f"\nMetrics saved to {METRICS_PATH}", flush=True)
    print(f"Model saved to {CHECKPOINT_PATH}", flush=True)

    if all_pass:
        print("\n✓ All acceptance criteria passed!", flush=True)
    else:
        print("\n✗ Some acceptance criteria failed. Consider training longer or adjusting hyperparameters.", flush=True)

    return model


# ═══════════════════════════════════════════════════════════════════════
#  Main
# ═══════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Train MemoryGAT model")
    parser.add_argument('--epochs', type=int, default=DEFAULT_EPOCHS)
    parser.add_argument('--batch-size', type=int, default=DEFAULT_BATCH_SIZE)
    parser.add_argument('--lr', type=float, default=DEFAULT_LR)
    parser.add_argument('--weight-decay', type=float, default=DEFAULT_WEIGHT_DECAY)
    parser.add_argument('--patience', type=int, default=DEFAULT_PATIENCE)
    parser.add_argument('--hidden-dim', type=int, default=128)
    parser.add_argument('--num-heads', type=int, default=8)
    parser.add_argument('--num-layers', type=int, default=3)
    parser.add_argument('--dropout', type=float, default=0.2)
    parser.add_argument('--no-mps', action='store_true', help='Disable MPS, use CPU')
    args = parser.parse_args()

    train(args)
