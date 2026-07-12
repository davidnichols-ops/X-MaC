#!/usr/bin/env python3

import argparse
import json
import random
import time
from pathlib import Path

import torch

from data_generator import DATA_DIR
from train import load_graphs, train

GNN_DIR = Path(__file__).resolve().parent
RESULTS_PATH = GNN_DIR / "sweep_results.json"
GRID = {
    "hidden_dim": [64, 128, 256],
    "num_heads": [4, 8, 16],
    "num_layers": [2, 3, 4],
    "dropout": [0.1, 0.2, 0.3],
    "lr": [0.001, 0.0005, 0.002],
    "batch_size": [32, 64, 128],
    "weight_decay": [0.001, 0.0001, 0.01],
    "w_class": [1.0, 3.0, 5.0],
}


def sample_config(rng, epochs, device):
    while True:
        config = {name: rng.choice(values) for name, values in GRID.items()}
        if config["hidden_dim"] % config["num_heads"] == 0:
            config.update({"epochs": epochs, "patience": epochs, "device": device})
            return config


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--configs", type=int, default=20)
    parser.add_argument("--epochs", type=int, default=100)
    parser.add_argument("--device", choices=("cpu", "mps", "auto"), default="auto")
    parser.add_argument("--train-limit", type=int)
    parser.add_argument("--val-limit", type=int)
    args = parser.parse_args()
    rng = random.Random(42)
    torch.manual_seed(42)
    train_graphs = load_graphs(DATA_DIR / "train.pt")
    val_graphs = load_graphs(DATA_DIR / "val.pt")
    if args.train_limit:
        train_graphs = train_graphs[:args.train_limit]
    if args.val_limit:
        val_graphs = val_graphs[:args.val_limit]
    results = []
    seen = set()
    while len(results) < args.configs:
        config = sample_config(rng, args.epochs, args.device)
        key = json.dumps(config, sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        index = len(results)
        started = time.perf_counter()
        checkpoint_path = GNN_DIR / "model" / f"sweep_{index:02d}.pt"
        checkpoint, history = train(config, train_graphs, val_graphs, checkpoint_path)
        best = checkpoint["val_metrics"]
        results.append({
            "config": config,
            "best_val_acc": best["acc"],
            "best_val_loss": best["loss"],
            "epoch_reached": checkpoint["epoch"],
            "time_seconds": time.perf_counter() - started,
            "checkpoint": str(checkpoint_path),
        })
        RESULTS_PATH.write_text(json.dumps({"configs": results}, indent=2))
    results.sort(key=lambda result: result["best_val_acc"], reverse=True)
    RESULTS_PATH.write_text(json.dumps({"configs": results}, indent=2))
    print(json.dumps(results[:3], indent=2))


if __name__ == "__main__":
    main()
