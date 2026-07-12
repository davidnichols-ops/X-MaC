"""GAT model for X-MaC file system graph analysis."""

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch_geometric.nn import GATConv


class GATModel(nn.Module):
    def __init__(self, num_features=16, hidden_dim=128, num_classes=27, num_heads=8,
                 num_layers=3, dropout=0.2):
        super().__init__()
        if hidden_dim % num_heads:
            raise ValueError("hidden_dim must be divisible by num_heads")
        if num_layers < 2:
            raise ValueError("num_layers must be at least 2")
        self.num_features = num_features
        self.hidden_dim = hidden_dim
        self.num_classes = num_classes
        self.num_heads = num_heads
        self.num_layers = num_layers
        self.dropout = dropout
        self.input_proj = nn.Linear(num_features, hidden_dim)
        self.convs = nn.ModuleList([
            GATConv(hidden_dim, hidden_dim // num_heads, heads=num_heads, dropout=dropout)
            for _ in range(num_layers)
        ])
        self.norms = nn.ModuleList([nn.LayerNorm(hidden_dim) for _ in range(num_layers)])
        self.safety_head = self._head(1)
        self.anomaly_head = self._head(1)
        self.class_head = self._head(num_classes)

    def _head(self, output_dim):
        return nn.Sequential(
            nn.Linear(self.hidden_dim, self.hidden_dim),
            nn.ReLU(),
            nn.Dropout(self.dropout),
            nn.Linear(self.hidden_dim, output_dim),
        )

    def forward(self, x, edge_index, batch=None):
        batched_shape = None
        if x.dim() == 3:
            batch_size, node_count, feature_count = x.shape
            batched_shape = (batch_size, node_count)
            x = x.reshape(batch_size * node_count, feature_count)
            if edge_index.dim() == 3:
                edge_index = torch.cat([
                    edge_index[index] + index * node_count for index in range(batch_size)
                ], dim=1)
        x = self.input_proj(x)
        for index, (conv, norm) in enumerate(zip(self.convs, self.norms)):
            identity = x
            x = F.elu(conv(x, edge_index))
            x = norm(x)
            if index < self.num_layers - 1:
                x = F.dropout(x, p=self.dropout, training=self.training)
                x = x + identity
        logits = self.class_head(x)
        safety = self.safety_head(x)
        anomaly = self.anomaly_head(x)
        if batched_shape is not None:
            batch_size, node_count = batched_shape
            logits = logits.reshape(batch_size, node_count, -1)
            safety = safety.reshape(batch_size, node_count, 1)
            anomaly = anomaly.reshape(batch_size, node_count, 1)
        return logits, safety, anomaly
