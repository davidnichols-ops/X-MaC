# GNN — Graph Neural Network

X-MaC includes **two** on-device Graph Neural Networks, both running entirely via CoreML — no network calls are made.

## Models

### 1. File Safety Scorer (XMacGNN)

Scores every scan finding by safety (0 = risky to clean, 1 = safe to clean).

```
Scan Findings → GraphBuilder → 3-Layer GAT → MLP → Sigmoid → Safety Scores (0-1)
```

- **Architecture:** 3-layer GAT, 128 hidden dim, 8 attention heads
- **Classes:** 27 (cache_file, build_output, log_file, source_code, trash, etc.)
- **Validation accuracy:** 99.76% (epoch 126)
- **Test accuracy:** 99.74%
- **Safety MAE:** 0.029
- **Anomaly MAE:** 0.016
- **Worst class:** build_output at 96.8% (confused with cargo_target)
- **Inference latency:** ~0.95ms for 200 nodes on CPU

### 2. Memory Optimization Model (XMacMemoryGNN)

Predicts optimal memory management actions (purge, suspend, terminate, etc.) based on process telemetry graphs.

```
Process Telemetry → GraphBuilder → 3-Layer GAT → Multi-Head → Actions + Risk + Growth + Pressure
```

- **Architecture:** 3-layer GAT, 128 hidden dim, 8 attention heads
- **Inputs:** Process nodes (24 features) + hardware/swap/compressor state
- **Outputs:** 6 action classes (no_action, pressure_relief, suggest_purge, deprioritize, suspend, terminate) + risk score + growth prediction + 3-class pressure level
- **Training data:** 10,000 synthetic graphs across 10 scenarios (healthy, memory_leak, cache_bloat, gpu_pressure, swap_thrashing, jetsam_cascade, startup_burst, idle_background, mixed_workload, random)
- **Acceptance criteria:** 85% action accuracy, 90% pressure accuracy, risk MAE ≤ 0.10, growth MAE ≤ 0.05, min per-class accuracy ≥ 80%
- **CoreML export:** Verified (CoreML vs PyTorch MAE: 0.0006 — essentially perfect)

## Nodes and Edges

**File GNN:**
- **Nodes** = findings (category, size, path depth, extension)
- **Edges** = same-directory, same-app, same-category relationships
- **Output** = per-node safety score (0 = risky, 1 = safe to clean)

**Memory GNN:**
- **Nodes** = processes (RSS, virtual size, threads, compressed bytes, CPU%, system flag) + hardware state
- **Edges** = parent-child process relationships, shared memory regions
- **Output** = per-process action recommendation + system-wide pressure prediction

## Directory Structure

```
gnn/
├── model/
│   ├── gnn.py              # GCN architecture (safety scoring)
│   ├── memory_gnn.py       # Memory optimization GNN
│   ├── xmac_gnn.pt         # Trained safety scoring model
│   └── memory_gnn.pt       # Trained memory optimization model
├── data/
│   ├── train.pt            # Training data (PyG format)
│   ├── val.pt              # Validation data
│   └── test.pt             # Test data
├── server/
│   └── inference_server.py # HTTP inference server (optional, for development)
├── train.py                # Train the safety scoring model
├── train_memory_gnn.py     # Train the memory optimization model
├── evaluate.py             # Evaluate model accuracy
├── export_coreml.py        # Export safety model to CoreML
├── export_memory_coreml.py # Export memory model to CoreML
├── data_generator.py       # Generate synthetic training data
├── memory_data_generator.py # Generate memory optimization training data
├── label_map.json          # Category → label index mapping
├── XMacGNN.mlpackage       # Pre-trained CoreML model (safety scoring)
└── XMacMemoryGNN.mlpackage # Pre-trained CoreML model (memory optimization)
```

## Training (Optional)

The pre-trained CoreML models are bundled with the app. You only need to retrain if you want to improve the model.

```bash
cd gnn
python -m venv .venv
source .venv/bin/activate
pip install torch torch-geometric numpy coremltools

# Generate training data
python data_generator.py

# Train the safety scoring model
python train.py

# Evaluate
python evaluate.py

# Export to CoreML
python export_coreml.py
```

## Dependencies

- Python 3.10+
- PyTorch 2.x
- torch-geometric
- numpy
- coremltools (for export only)

## Adding New Categories

1. Add the category to `label_map.json`
2. Update `data_generator.py` to generate data for the new category
3. Retrain the model
4. Re-export to CoreML
5. Update the GUI's `CoreMLGNN.swift` if needed
