# GNN — Graph Neural Network

On-device safety scoring for scan findings using a Graph Neural Network (GNN).

## Overview

X-MaC uses a GNN to score every scan finding by safety (0 = risky to clean, 1 = safe to clean). The model runs entirely on-device via CoreML — no network calls are made.

```
Scan Findings → GraphBuilder → 3-Layer GCN → MLP → Sigmoid → Safety Scores (0-1)
```

- **Nodes** = findings (category, size, path depth, extension)
- **Edges** = same-directory, same-app, same-category relationships
- **Output** = per-node safety score (0 = risky, 1 = safe to clean)

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
