"""MemoryGAT model for macOS unified memory optimization.

Extends the filesystem GAT architecture for the memory domain:
- Heterogeneous graph (processes, hardware, swap, compressor)
- Edge-type-aware attention
- 4 output heads: pressure, growth, action, risk
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch_geometric.nn import GATConv, global_mean_pool


# Feature dimensions (must match Rust + data_generator)
PROCESS_FEATURE_DIM = 24
HARDWARE_FEATURE_DIM = 8
SWAP_FEATURE_DIM = 6
COMPRESSOR_FEATURE_DIM = 6
MAX_FEATURE_DIM = PROCESS_FEATURE_DIM  # 24

HIDDEN_DIM = 128
NUM_HEADS = 8
NUM_LAYERS = 3
DROPOUT = 0.2

# Output dimensions
NUM_ACTIONS = 6   # no_action, pressure_relief, suggest_purge, deprioritize, suspend, terminate
NUM_PRESSURE = 3  # normal, warn, critical


class MemoryGAT(nn.Module):
    """Graph Attention Network for memory system optimization.

    Input: Heterogeneous memory graph with variable-length features per node type.
    Output: Per-process predictions (action, risk, growth) + system pressure.
    """

    def __init__(
        self,
        hidden_dim: int = HIDDEN_DIM,
        num_heads: int = NUM_HEADS,
        num_layers: int = NUM_LAYERS,
        dropout: float = DROPOUT,
        num_actions: int = NUM_ACTIONS,
        num_pressure: int = NUM_PRESSURE,
    ):
        super().__init__()
        if hidden_dim % num_heads:
            raise ValueError("hidden_dim must be divisible by num_heads")
        if num_layers < 2:
            raise ValueError("num_layers must be at least 2")

        self.hidden_dim = hidden_dim
        self.num_heads = num_heads
        self.num_layers = num_layers
        self.dropout = dropout

        # Input projection: project each node type's features to hidden_dim
        # Since we pad all features to MAX_FEATURE_DIM, a single projection works
        self.input_proj = nn.Linear(MAX_FEATURE_DIM, hidden_dim)

        # GAT layers
        self.convs = nn.ModuleList([
            GATConv(hidden_dim, hidden_dim // num_heads, heads=num_heads, dropout=dropout)
            for _ in range(num_layers)
        ])
        self.norms = nn.ModuleList([nn.LayerNorm(hidden_dim) for _ in range(num_layers)])

        # Output heads
        self.action_head = self._make_head(num_actions)
        self.risk_head = self._make_head(1)
        self.growth_head = self._make_head(1)
        self.pressure_head = self._make_head(num_pressure)

    def _make_head(self, output_dim: int) -> nn.Sequential:
        return nn.Sequential(
            nn.Linear(self.hidden_dim, self.hidden_dim),
            nn.ReLU(),
            nn.Dropout(self.dropout),
            nn.Linear(self.hidden_dim, output_dim),
        )

    def forward(self, x, edge_index, batch=None):
        """Forward pass.

        Args:
            x: Node features [N, MAX_FEATURE_DIM] (padded to 24 dims)
            edge_index: Edge indices [2, E]
            batch: Batch assignment [N] (for system-level predictions)

        Returns:
            action_logits: [N, num_actions] — per-process action recommendation
            risk_scores: [N, 1] — per-process risk score (0=safe, 1=dangerous)
            growth_preds: [N, 1] — per-process predicted RSS growth (normalized)
            pressure_logits: [num_graphs, num_pressure] — system pressure prediction
        """
        batched_shape = None
        if x.dim() == 3:
            batch_size, node_count, feature_count = x.shape
            batched_shape = (batch_size, node_count)
            x = x.reshape(batch_size * node_count, feature_count)
            if edge_index.dim() == 3:
                edge_index = torch.cat([
                    edge_index[i] + i * node_count for i in range(batch_size)
                ], dim=1)

        # Pad features to MAX_FEATURE_DIM if needed
        if x.size(-1) < MAX_FEATURE_DIM:
            pad = torch.zeros(x.size(0), MAX_FEATURE_DIM - x.size(-1), device=x.device)
            x = torch.cat([x, pad], dim=-1)

        # Input projection
        x = self.input_proj(x)

        # GAT layers with residual connections
        for i, (conv, norm) in enumerate(zip(self.convs, self.norms)):
            identity = x
            x = F.elu(conv(x, edge_index))
            x = norm(x)
            if i < self.num_layers - 1:
                x = F.dropout(x, p=self.dropout, training=self.training)
                x = x + identity

        # Per-node predictions
        action_logits = self.action_head(x)
        risk_scores = torch.sigmoid(self.risk_head(x))
        growth_preds = self.growth_head(x)

        # System-level pressure prediction (pool all nodes)
        if batch is not None:
            pooled = global_mean_pool(x, batch)
        else:
            pooled = x.mean(dim=0, keepdim=True)
        pressure_logits = self.pressure_head(pooled)

        if batched_shape is not None:
            batch_size, node_count = batched_shape
            action_logits = action_logits.reshape(batch_size, node_count, -1)
            risk_scores = risk_scores.reshape(batch_size, node_count, 1)
            growth_preds = growth_preds.reshape(batch_size, node_count, 1)
            pressure_logits = pressure_logits.reshape(batch_size, -1)

        return action_logits, risk_scores, growth_preds, pressure_logits
