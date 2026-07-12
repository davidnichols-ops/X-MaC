# X-MaC GNN Training — North Star Specification

> **Goal: Achieve ≥95% validation accuracy on node classification.**
> This document is not finished until that number is hit and the model is exported to CoreML.
> Any AI agent picking this up should treat 95% as the hard gate. Do not stop, do not declare
> success, do not commit until `val_acc >= 0.95` is printed in the training log AND verified
> on a held-out test set that the model has never seen.

---

## 0. Current State (as of this writing)

| Metric | Value |
|--------|-------|
| Validation accuracy | ~26.5% |
| Model | GAT, 2 layers, 64 hidden, 4 heads |
| Features | 9-dim per node |
| Classes | 27 |
| Training data | 500 synthetic graphs, 80 nodes each |
| Epochs | 200 |
| Export | CoreML (.mlpackage) via distillation MLP |

### Root Cause Analysis — Why accuracy is 26.5%

1. **Graph structure is noise.** `generate_graph()` picks a random category for each node and connects it to a random earlier node. Real filesystems have structural patterns: cache dirs contain cache files, build dirs contain .o/.rlib files, log dirs contain .log files. The GNN cannot learn parent-child category correlations because there are none.

2. **Class sampling is uniform.** `random.choice(CATEGORIES)` gives every class equal probability. Real filesystems are dominated by `file`, `directory`, `source_code`, `config_file`. `trash` and `disk_image` are rare. The model sees too few examples of rare classes and too many of common ones relative to reality.

3. **Feature vector is too sparse.** 9 features with no extension type, no parent directory name signal, no sibling count, no file permission bits. The model has to distinguish 27 classes from 9 numbers, several of which are binary flags.

4. **No real filesystem data.** Training is 100% synthetic with no ground-truth labels from actual scans. The model learns the synthetic distribution, not the real one.

5. **Model is undersized.** 64 hidden dim × 2 GAT layers for 27 classes with 9 features. The capacity is marginal.

6. **No class weighting.** Cross-entropy loss treats all classes equally, but the synthetic data is uniform so this isn't the issue — the issue is that real data won't be uniform.

7. **No augmentation.** No graph augmentation (edge dropout, node masking, subgraph sampling).

8. **No hyperparameter search.** Single config, no sweeps.

---

## 1. Phase 1 — Realistic Data Generation

### 1.1 Build a Tree-Structured Graph Generator

Replace the random-parent assignment with a **realistic filesystem tree generator** that models actual directory structures. Each graph should simulate a real filesystem subtree.

**Directory archetypes** (each generates children with the correct category distribution):

```
root/                          # label: root
├── Users/<user>/              # label: directory
│   ├── Library/Caches/        # label: cache_dir
│   │   ├── com.apple.Safari/  # label: cache_dir
│   │   │   ├── *.cache        # label: cache_file
│   │   │   └── ...
│   │   ├── com.google.Chrome/ # label: cache_dir
│   │   └── ...
│   ├── Library/Logs/          # label: log_dir
│   │   ├── *.log              # label: log_file
│   │   └── ...
│   ├── Downloads/             # label: directory
│   │   ├── *.dmg              # label: disk_image
│   │   ├── *.zip              # label: archive
│   │   └── ...
│   ├── Documents/             # label: directory
│   │   ├── *.pdf              # label: document
│   │   └── ...
│   ├── .Trash/                # label: trash (dir)
│   │   └── *                  # label: trash
│   └── Projects/              # label: directory
│       ├── myapp/             # label: directory
│       │   ├── src/           # label: directory
│       │   │   ├── *.rs       # label: source_code
│       │   │   └── *.swift    # label: source_code
│       │   ├── target/        # label: cargo_target (dir)
│       │   │   ├── debug/     # label: build_output (dir)
│       │   │   │   ├── *.rlib # label: cargo_target
│       │   │   │   └── myapp  # label: executable
│       │   │   └── release/
│       │   ├── build/         # label: build_output (dir)
│       │   │   ├── *.o        # label: build_output
│       │   │   └── ...
│       │   ├── __pycache__/   # label: python_cache (dir)
│       │   │   └── *.pyc      # label: python_cache
│       │   ├── .git/          # label: git_dir
│       │   │   └── ...
│       │   ├── config.json    # label: config_file
│       │   └── Package.swift  # label: source_code
│       └── ...
├── Applications/              # label: directory
│   ├── Xcode.app/             # label: app_bundle
│   │   └── Contents/
│   │       └── ...
│   └── ...
├── usr/lib/                   # label: library_dir
│   ├── *.dylib                # label: library_file
│   └── ...
├── var/log/                   # label: log_dir
│   └── *.log                  # label: log_file
├── private/var/folders/       # label: temp_file (dir)
│   └── *.tmp                  # label: temp_file
└── System/                    # label: directory (protected)
    └── ...
```

**Implementation requirements:**

- Each directory archetype has a **weighted child generator** that produces children of the correct categories with realistic frequencies
- Depth is determined by the archetype, not random
- Sizes follow category-specific distributions (log-normal, not uniform)
- Hidden files (`.git`, `.Trash`, `__pycache__`) have `is_hidden=True`
- Extensions match the category (`.rs` → source_code, `.dylib` → library_file, etc.)
- Age/access patterns are category-specific (cache files are recently accessed, log files are old, trash files are old)
- Sibling edges connect nodes in the same directory (not random nodes)

### 1.2 Expand Feature Vector from 9 to 16 Dimensions

Add these features to both the Python generator AND the Rust extractor (`src/engines/graph/extractor.rs`):

| Index | Feature | Description | Range |
|-------|---------|-------------|-------|
| 0 | log_size | log(size_bytes + 1) / 30.0 | [0, 1] |
| 1 | depth_norm | depth / max_depth | [0, 1] |
| 2 | is_dir | 1.0 if directory | {0, 1} |
| 3 | is_file | 1.0 if file | {0, 1} |
| 4 | is_symlink | 1.0 if symlink | {0, 1} |
| 5 | is_hidden | 1.0 if name starts with . | {0, 1} |
| 6 | age_days_norm | min(age_days / 365, 1.0) | [0, 1] |
| 7 | access_age_days_norm | min(access_days / 365, 1.0) | [0, 1] |
| 8 | has_extension | 1.0 if extension present | {0, 1} |
| 9 | **num_children_norm** | min(num_children / 50, 1.0) | [0, 1] |
| 10 | **is_executable** | 1.0 if executable bit set or .app | {0, 1} |
| 11 | **is_archive_ext** | 1.0 if ext in {zip,gz,tar,dmg,iso,rar,7z} | {0, 1} |
| 12 | **is_code_ext** | 1.0 if ext in {rs,swift,py,js,ts,c,cpp,h,go,rb,java} | {0, 1} |
| 13 | **is_media_ext** | 1.0 if ext in {png,jpg,jpeg,mp4,mov,mp3,aac,pdf} | {0, 1} |
| 14 | **is_config_ext** | 1.0 if ext in {json,yaml,yml,toml,xml,plist,conf,ini,env} | {0, 1} |
| 15 | **parent_is_dir** | 1.0 if parent node is a directory | {0, 1} |

**IMPORTANT:** You MUST update `NUM_FEATURES = 16` in `gnn/train.py`, `gnn/model/gnn.py`, the Rust extractor, and the CoreML export. The feature vector must be consistent across Python training, Rust extraction, and CoreML inference.

### 1.3 Generate 10,000+ Graphs with Class Balance

| Parameter | Old | New |
|-----------|-----|-----|
| NUM_SYNTHETIC_GRAPHS | 500 | 10,000 |
| NODES_PER_GRAPH | 80 | 50–200 (variable) |
| Train/Val/Test split | 80/20 | 70/15/15 |

**Class balancing:** Use weighted sampling so that rare classes (trash, disk_image, backup_dir) appear in at least 5% of graphs. Common classes (file, directory, source_code) should appear in 80%+ of graphs.

**Ground truth labels:** Every node gets its true label from the archetype that generated it. No noise on labels — the noise goes into features (size, age, access time).

### 1.4 Real Filesystem Sampling (Optional but High Impact)

If possible, add a script `gnn/scan_real.py` that:
1. Uses the Rust binary to extract graphs from real directories (`~/Library/Caches`, `~/Projects`, `/var/log`, `/usr/lib`)
2. Labels each node using the rules engine (`src/engines/clean/rules.rs`) as ground truth
3. Saves as `.pt` files in `gnn/data/real/`
4. Mixes real + synthetic data 50/50 in training

This bridges the sim-to-real gap. Even 50 real graphs would dramatically help.

---

## 2. Phase 2 — Model Architecture Upgrade

### 2.1 Upgrade to 3-Layer GAT with Residual Connections

```python
class GATModelV2(nn.Module):
    def __init__(self, num_features=16, hidden_dim=128, num_classes=27, num_heads=8):
        super().__init__()
        # Input projection
        self.input_proj = nn.Linear(num_features, hidden_dim)

        # 3 GAT layers with residual connections
        self.conv1 = GATConv(hidden_dim, hidden_dim // num_heads, heads=num_heads, dropout=0.2)
        self.norm1 = nn.LayerNorm(hidden_dim)

        self.conv2 = GATConv(hidden_dim, hidden_dim // num_heads, heads=num_heads, dropout=0.2)
        self.norm2 = nn.LayerNorm(hidden_dim)

        self.conv3 = GATConv(hidden_dim, hidden_dim // num_heads, heads=num_heads, dropout=0.2)
        self.norm3 = nn.LayerNorm(hidden_dim)

        # Heads
        self.safety_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(hidden_dim, 1)
        )
        self.anomaly_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(hidden_dim, 1)
        )
        self.class_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(hidden_dim, num_classes)
        )

    def forward(self, x, edge_index, batch=None):
        x = self.input_proj(x)

        # Layer 1 with residual
        identity = x
        x = F.elu(self.conv1(x, edge_index))
        x = self.norm1(x)
        x = F.dropout(x, p=0.2, training=self.training)
        x = x + identity  # residual

        # Layer 2 with residual
        identity = x
        x = F.elu(self.conv2(x, edge_index))
        x = self.norm2(x)
        x = F.dropout(x, p=0.2, training=self.training)
        x = x + identity

        # Layer 3 (no residual, output layer)
        x = F.elu(self.conv3(x, edge_index))
        x = self.norm3(x)

        safety = self.safety_head(x)
        anomaly = self.anomaly_head(x)
        logits = self.class_head(x)

        return logits, safety, anomaly
```

### 2.2 Architecture Changes Summary

| Parameter | Old | New |
|-----------|-----|-----|
| num_features | 9 | 16 |
| hidden_dim | 64 | 128 |
| num_heads | 4 | 8 |
| GAT layers | 2 | 3 |
| Residual connections | No | Yes |
| Layer normalization | No | Yes |
| Dropout | 0.1 | 0.2 |
| Input projection | No | Yes (Linear) |

---

## 3. Phase 3 — Training Pipeline Overhaul

### 3.1 Loss Function with Class Weighting

```python
# Compute class weights from training data
from collections import Counter
label_counts = Counter()
for g in train_graphs:
    for label in g.y.tolist():
        label_counts[label] += 1

# Inverse frequency weighting
total = sum(label_counts.values())
class_weights = torch.zeros(NUM_CLASSES)
for i in range(NUM_CLASSES):
    class_weights[i] = total / (NUM_CLASSES * max(label_counts.get(i, 1), 1))

class_weights = class_weights / class_weights.mean()  # normalize

loss_class = F.cross_entropy(logits, batch.y, weight=class_weights.to(device))
```

### 3.2 Learning Rate Schedule

```python
optimizer = torch.optim.AdamW(model.parameters(), lr=0.001, weight_decay=1e-3)
scheduler = torch.optim.lr_scheduler.CosineAnnealingWarmRestarts(optimizer, T_0=50, T_mult=2)
```

- Use AdamW (not Adam) for better weight decay
- Cosine annealing with warm restarts: restart at epoch 50, 100, 200, 400
- Weight decay 1e-3 (up from 1e-4)

### 3.3 Loss Weights

```python
W_CLASS = 3.0    # up from 1.0 — classification is the primary objective
W_SAFETY = 1.0   # down from 2.0 — safety is secondary
W_ANOMALY = 0.5  # down from 1.0 — anomaly is tertiary
```

### 3.4 Training Duration

| Parameter | Old | New |
|-----------|-----|-----|
| EPOCHS | 200 | 500 |
| BATCH_SIZE | 32 | 64 |
| LR | 0.001 (constant) | 0.001 (cosine annealing) |
| Early stopping | No | Yes (patience=100, monitor val_acc) |

### 3.5 Data Augmentation

Apply random augmentations during training (not validation):

```python
def augment_graph(data):
    # 1. Random edge dropout (20% of edges)
    if random.random() < 0.5:
        edge_mask = torch.rand(data.edge_index.size(1)) > 0.2
        data.edge_index = data.edge_index[:, edge_mask]

    # 2. Random node feature masking (10% of features)
    if random.random() < 0.5:
        mask = torch.rand(data.x.size(0), data.x.size(1)) < 0.1
        data.x[mask] = 0.0

    # 3. Random subgraph sampling (take 70-100% of nodes)
    if random.random() < 0.3:
        keep_ratio = random.uniform(0.7, 1.0)
        num_keep = int(data.x.size(0) * keep_ratio)
        keep_idx = torch.randperm(data.x.size(0))[:num_keep]
        data = data.subgraph(keep_idx)

    return data
```

### 3.6 Metrics to Track

For every epoch, log:

```
Epoch {epoch}/{epochs}  lr={lr:.6f}
  train_loss={train_loss:.4f}  train_acc={train_acc:.4f}
  val_loss={val_loss:.4f}  val_acc={val_acc:.4f}
  val_safety_mae={val_safety_mae:.4f}  val_anomaly_mae={val_anomaly_mae:.4f}
  per_class_acc: {per_class_accuracy}
  worst_classes: {bottom_5_classes_by_accuracy}
  confusion_matrix: {saved to file every 50 epochs}
```

Track per-class accuracy. If any class is below 80%, that's a signal to increase its weight or generate more examples.

---

## 4. Phase 4 — Hyperparameter Sweep

Run a systematic sweep. For each config, train for 100 epochs and record best val_acc. Take the top 3 configs and train each for 500 epochs.

### Sweep Grid

| Hyperparameter | Values to Try |
|----------------|---------------|
| hidden_dim | 64, 128, 256 |
| num_heads | 4, 8, 16 |
| num_gat_layers | 2, 3, 4 |
| dropout | 0.1, 0.2, 0.3 |
| lr | 0.001, 0.0005, 0.002 |
| batch_size | 32, 64, 128 |
| weight_decay | 1e-3, 1e-4, 1e-2 |
| W_CLASS | 1.0, 3.0, 5.0 |

**Strategy:** Use random search (not grid search) — sample 20 random configs from the grid. Train each for 100 epochs. Take the top 3 by val_acc and train for 500 epochs each.

### Sweep Logging

Save results to `gnn/sweep_results.json`:

```json
{
  "configs": [
    {
      "config": {"hidden_dim": 128, "num_heads": 8, ...},
      "best_val_acc": 0.89,
      "best_val_loss": 0.34,
      "epoch_reached": 87,
      "time_seconds": 234
    },
    ...
  ]
}
```

---

## 5. Phase 5 — Evaluation and Verification

### 5.1 Test Set Evaluation

After selecting the best model, evaluate on the **test set** (the 15% that was never seen during training or hyperparameter tuning):

```python
model.eval()
correct = 0
total = 0
per_class_correct = [0] * NUM_CLASSES
per_class_total = [0] * NUM_CLASSES

with torch.no_grad():
    for batch in test_loader:
        logits, safety, anomaly = model(batch.x, batch.edge_index)
        pred = logits.argmax(dim=1)
        correct += (pred == batch.y).sum().item()
        total += batch.y.size(0)
        for i in range(batch.y.size(0)):
            per_class_total[batch.y[i]] += 1
            if pred[i] == batch.y[i]:
                per_class_correct[batch.y[i]] += 1

test_acc = correct / total
print(f"TEST ACCURACY: {test_acc:.4f}")
print(f"PER-CLASS ACCURACY:")
for i in range(NUM_CLASSES):
    if per_class_total[i] > 0:
        acc = per_class_correct[i] / per_class_total[i]
        label = IDX_TO_LABEL[i]
        print(f"  {label:30s}  {acc:.4f}  ({per_class_correct[i]}/{per_class_total[i]})")
```

### 5.2 Acceptance Criteria (ALL must pass)

1. **Overall test accuracy ≥ 95%**
2. **Per-class accuracy ≥ 85% for every class** (no class can be ignored)
3. **Safety MAE ≤ 0.08** (safety scores must be close to ground truth)
4. **Anomaly MAE ≤ 0.10**
5. **No confusion between safe-to-delete and not-safe-to-delete classes** (cache_dir vs config_file confusion must be < 2%)
6. **CoreML export succeeds** and the distilled MLP matches GNN outputs with MAE ≤ 0.05
7. **Inference latency < 50ms** for a 200-node graph on CPU

### 5.3 Confusion Matrix Analysis

Generate a confusion matrix heatmap and save to `gnn/confusion_matrix.png`:

```python
from sklearn.metrics import confusion_matrix, classification_report
import seaborn as sns
import matplotlib.pyplot as plt

cm = confusion_matrix(all_labels, all_preds, labels=list(range(NUM_CLASSES)))
plt.figure(figsize=(14, 12))
sns.heatmap(cm, annot=True, fmt='d', xticklabels=list(LABEL_MAP.keys()), yticklabels=list(LABEL_MAP.keys()))
plt.title(f'Confusion Matrix — Test Accuracy: {test_acc:.4f}')
plt.tight_layout()
plt.savefig('gnn/confusion_matrix.png', dpi=150)
```

Also save `gnn/classification_report.txt` with the full sklearn classification report.

---

## 6. Phase 6 — CoreML Export

### 6.1 Distillation MLP Upgrade

The CoreML export uses a per-node MLP distilled from the GNN. Upgrade it:

```python
class NodeMLPv2(nn.Module):
    def __init__(self, in_features=16, hidden=128, num_classes=27):
        super().__init__()
        self.fc1 = nn.Linear(in_features, hidden)
        self.fc2 = nn.Linear(hidden, hidden)
        self.fc3 = nn.Linear(hidden, hidden)
        self.norm1 = nn.LayerNorm(hidden)
        self.norm2 = nn.LayerNorm(hidden)
        self.safety_head = nn.Sequential(
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden, 1)
        )
        self.anomaly_head = nn.Sequential(
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden, 1)
        )
        self.class_head = nn.Sequential(
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden, num_classes)
        )

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
        logits = self.class_head(x)
        return torch.cat([logits, safety, anomaly], dim=1)
```

### 6.2 Distillation Training

- Generate 10,000 distillation samples (not 50 graphs worth)
- Train MLP for 500 epochs (not 100)
- Use learning rate 0.001 with cosine annealing
- Target distillation MAE ≤ 0.05

### 6.3 CoreML Model Verification

After export, verify the CoreML model:

```python
import coremltools as ct

mlmodel = ct.models.MLModel(str(COREML_EXPORT_PATH))

# Test with a known input
test_input = torch.tensor([[0.5, 0.3, 0.0, 1.0, 0.0, 1.0, 0.8, 0.9, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]])
pred = mlmodel.predict({"node_features": test_input.numpy()})
print(f"CoreML output shape: {pred['predictions'].shape}")
print(f"CoreML output: {pred['predictions']}")
```

---

## 7. Phase 7 — Rust Integration Update

### 7.1 Update Feature Extractor

Update `src/engines/graph/extractor.rs` to produce 16 features instead of 9. The new features:

```rust
let features = vec![
    // Existing 9 features (unchanged)
    if size_bytes > 0 { (size_bytes as f32).ln() / 30.0 } else { 0.0 },  // 0: log_size
    depth as f32 / self.max_depth as f32,                                  // 1: depth_norm
    if node_type == NodeType::Directory { 1.0 } else { 0.0 },             // 2: is_dir
    if node_type == NodeType::File { 1.0 } else { 0.0 },                  // 3: is_file
    if node_type == NodeType::Symlink { 1.0 } else { 0.0 },               // 4: is_symlink
    if is_hidden { 1.0 } else { 0.0 },                                     // 5: is_hidden
    age_days_norm,                                                          // 6: age_days_norm
    access_age_days_norm,                                                   // 7: access_age_days_norm
    if extension.is_some() { 1.0 } else { 0.0 },                           // 8: has_extension
    // New 7 features
    min(num_children as f32 / 50.0, 1.0),                                  // 9: num_children_norm
    if is_executable { 1.0 } else { 0.0 },                                 // 10: is_executable
    if is_archive_ext { 1.0 } else { 0.0 },                                // 11: is_archive_ext
    if is_code_ext { 1.0 } else { 0.0 },                                   // 12: is_code_ext
    if is_media_ext { 1.0 } else { 0.0 },                                  // 13: is_media_ext
    if is_config_ext { 1.0 } else { 0.0 },                                 // 14: is_config_ext
    if parent_is_dir { 1.0 } else { 0.0 },                                 // 15: parent_is_dir
];
```

You'll need to:
- Track `num_children` (already done)
- Check executable bit via `std::os::unix::fs::PermissionsExt`
- Match extension against the archive/code/media/config sets
- Track whether parent is a directory (pass this down during the walk)

### 7.2 Update NUM_FEATURES

Update all references from 9 to 16:
- `gnn/train.py`: `NUM_FEATURES = 16`
- `gnn/model/gnn.py`: default `num_features=16`
- `gnn/server/inference_server.py`: default `num_features=16`
- `src/engines/graph/extractor.rs`: feature vector length
- `gui/XMacApp/Sources/XMacApp/CoreMLGNN.swift`: input shape
- Any test assertions that check `num_features == 9`

---

## 8. Execution Order

Follow this exact sequence. Do not skip steps. Do not move to the next phase until the current one is verified.

### Step 1: Data Generation (Phase 1)
- [ ] Write new tree-structured graph generator
- [ ] Generate 10,000 graphs
- [ ] Verify class distribution is balanced
- [ ] Verify graph structure is realistic (print a sample graph as a tree)
- [ ] Save dataset to `gnn/data/train.pt`, `gnn/data/val.pt`, `gnn/data/test.pt`

### Step 2: Feature Expansion (Phase 1.2 + Phase 7)
- [ ] Add 7 new features to Python generator
- [ ] Add 7 new features to Rust extractor
- [ ] Update NUM_FEATURES everywhere
- [ ] Run `cargo test` to verify Rust changes
- [ ] Verify feature vectors are 16-dim in both Python and Rust

### Step 3: Model Upgrade (Phase 2)
- [ ] Implement GATModelV2 with 3 layers, residuals, layer norm
- [ ] Verify forward pass works with 16 features
- [ ] Verify output shapes are correct

### Step 4: Training Pipeline (Phase 3)
- [ ] Implement class-weighted loss
- [ ] Implement AdamW + cosine annealing scheduler
- [ ] Implement data augmentation
- [ ] Implement early stopping
- [ ] Implement per-class accuracy logging
- [ ] Run a 50-epoch sanity check — val_acc should be > 60%

### Step 5: Full Training Run (Phase 3)
- [ ] Train for 500 epochs
- [ ] Log metrics every epoch
- [ ] Save best model checkpoint
- [ ] Check: is val_acc ≥ 90%? If not, go to Phase 4

### Step 6: Hyperparameter Sweep (Phase 4) — only if needed
- [ ] Run 20 random configs for 100 epochs each
- [ ] Select top 3 configs
- [ ] Train top 3 for 500 epochs each
- [ ] Select best model

### Step 7: Evaluation (Phase 5)
- [ ] Evaluate on test set
- [ ] Generate confusion matrix
- [ ] Generate classification report
- [ ] Check ALL acceptance criteria pass
- [ ] If any criterion fails, identify the failing class/issue and go back to Step 1 (generate more data for weak classes) or Step 6 (tune hyperparameters)

### Step 8: CoreML Export (Phase 6)
- [ ] Train distillation MLP (500 epochs, 10k samples)
- [ ] Export to CoreML
- [ ] Verify CoreML model output matches PyTorch output
- [ ] Verify model size < 5MB

### Step 9: Rust Integration (Phase 7)
- [ ] Update Rust extractor with 16 features
- [ ] Run `cargo build` and `cargo test`
- [ ] Update Swift CoreML integration if needed
- [ ] End-to-end test: extract graph from real directory → run through model → verify predictions

### Step 10: Final Verification
- [ ] `val_acc >= 0.95` in training log
- [ ] `test_acc >= 0.95` in test evaluation
- [ ] All per-class accuracies ≥ 85%
- [ ] CoreML model exported and verified
- [ ] Rust builds and tests pass
- [ ] Commit and push

---

## 9. File Manifest

Files to create or modify:

### New files
- `gnn/data_generator.py` — Tree-structured graph generator
- `gnn/data_generator_v2.py` — If v1 doesn't reach 95%, iterate
- `gnn/scan_real.py` — Real filesystem scanner (optional)
- `gnn/sweep.py` — Hyperparameter sweep script
- `gnn/evaluate.py` — Test set evaluation + confusion matrix
- `gnn/sweep_results.json` — Sweep results
- `gnn/confusion_matrix.png` — Final confusion matrix
- `gnn/classification_report.txt` — Final classification report
- `gnn/data/train.pt` — Training dataset
- `gnn/data/val.pt` — Validation dataset
- `gnn/data/test.pt` — Test dataset

### Modified files
- `gnn/train.py` — New model, new training loop, new hyperparameters
- `gnn/model/gnn.py` — GATModelV2 with 16 features, 128 hidden, 3 layers
- `gnn/server/inference_server.py` — Update for 16 features
- `gnn/label_map.json` — Unchanged (27 classes)
- `src/engines/graph/extractor.rs` — 16 features
- `gui/XMacApp/Sources/XMacApp/CoreMLGNN.swift` — 16 input features

---

## 10. Expected Accuracy Trajectory

Based on the root cause analysis, here's the expected improvement at each phase:

| Phase | Expected Val Acc | Why |
|-------|-----------------|-----|
| Current (baseline) | 26.5% | Random graph structure, 9 features, undersized model |
| After Phase 1 (realistic data) | 60–70% | Tree structure gives GNN real signal to learn from |
| After Phase 1.2 (16 features) | 70–80% | Extension type and parent context are strong signals |
| After Phase 2 (bigger model) | 80–88% | More capacity to learn 27-way classification |
| After Phase 3 (better training) | 88–93% | Class weighting, augmentation, LR schedule |
| After Phase 4 (sweep) | 93–96% | Optimal hyperparameters |
| After Phase 5 (evaluation) | ≥95% | Final model on test set |

If you're not on this trajectory, something is wrong. Debug:
- Below 60% after Phase 1 → data generator is broken, check graph structure
- Below 80% after Phase 1.2 → features aren't being computed correctly
- Below 88% after Phase 2 → model isn't learning, check gradients and loss
- Below 93% after Phase 3 → training dynamics issue, check LR and loss weights
- Below 95% after Phase 4 → need more data or better augmentation, iterate

---

## 11. Hard Constraints

1. **DO NOT change the label_map.json** — 27 classes, fixed mapping. The Rust side depends on it.
2. **DO NOT change the CoreML input/output interface** — Swift expects `node_features` input and `predictions` output.
3. **DO NOT reduce the number of classes** — even if some are rare. All 27 must work.
4. **DO NOT skip the test set** — val_acc is not enough. Test on unseen data.
5. **DO NOT use the test set for model selection** — only val set. Test set is final.
6. **DO NOT export to CoreML until test_acc ≥ 95%** — no point exporting a bad model.
7. **DO NOT change the Rust GraphNode/GraphEdge/FileSystemGraph structs** — only the feature vector length changes (9 → 16).
8. **DO NOT remove the safety/anomaly heads** — they're part of the model's value proposition.
9. **DO NOT stop until ALL acceptance criteria in Phase 5 pass.**
10. **DO NOT declare success without printing the test accuracy and per-class breakdown.**

---

## 12. Environment Setup

```bash
# Python environment
cd /Users/david/Projects/X-MaC
python3 -m venv .venv
source .venv/bin/activate
pip install torch torch-geometric coremltools scikit-learn seaborn matplotlib fastapi uvicorn

# Verify GPU availability (optional — CPU is fine for this scale)
python3 -c "import torch; print(f'CUDA: {torch.cuda.is_available()}, MPS: {torch.backends.mps.is_available()}')"

# Rust
cargo build
cargo test
```

If MPS (Apple Silicon GPU) is available, use it:
```python
device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
```

---

## 13. Success Definition

The task is complete when ALL of the following are true:

1. `gnn/train.py` prints `TEST ACCURACY: 0.9xxx` (≥ 0.95)
2. `gnn/classification_report.txt` shows ≥ 0.85 recall for every class
3. `gnn/confusion_matrix.png` shows a strong diagonal with minimal off-diagonal mass
4. `gnn/XMacGNN.mlpackage` exists and is < 5MB
5. `cargo build` and `cargo test` pass with 16-feature extractor
6. The changes are committed and pushed

**Print this at the end:**

```
╔══════════════════════════════════════════════════════════╗
║  X-MaC GNN Training — COMPLETE                          ║
║                                                          ║
║  Test Accuracy:   {test_acc:.4f}                         ║
║  Safety MAE:      {safety_mae:.4f}                       ║
║  Anomaly MAE:     {anomaly_mae:.4f}                      ║
║  CoreML Export:   {yes/no}                               ║
║  Rust Tests:      {pass/fail}                            ║
║                                                          ║
║  North Star (≥95%): {ACHIEVED / NOT ACHIEVED}            ║
╚══════════════════════════════════════════════════════════╝
```

If it says NOT ACHIEVED, go back and iterate. This document is not finished until it says ACHIEVED.
