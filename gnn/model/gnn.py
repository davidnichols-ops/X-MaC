"""
GAT (Graph Attention Network) model for X-MaC file system graph analysis.
Predicts safety_score (safe to delete) and anomaly_score (unusual file).
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch_geometric.nn import GATConv, global_mean_pool


class GATModel(nn.Module):
    def __init__(self, num_features=9, hidden_dim=64, num_classes=27, num_heads=4):
        super().__init__()
        self.conv1 = GATConv(num_features, hidden_dim, heads=num_heads, dropout=0.1)
        self.conv2 = GATConv(hidden_dim * num_heads, hidden_dim, heads=1, dropout=0.1)

        # Safety score head (regression -> sigmoid)
        self.safety_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden_dim // 2, 1)
        )

        # Anomaly score head (regression -> sigmoid)
        self.anomaly_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(hidden_dim // 2, 1)
        )

        # Classification head (node type prediction)
        self.class_head = nn.Sequential(
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Linear(hidden_dim // 2, num_classes)
        )

    def forward(self, x, edge_index, batch=None):
        # x: [B, N, F] or [N, F]
        if x.dim() == 3:
            B, N, F = x.shape
            x = x.view(B * N, F)
            if edge_index.dim() == 3:
                # Multiple graphs
                edge_indices = []
                offset = 0
                for i in range(B):
                    edge_indices.append(edge_index[i] + offset)
                    offset += N
                edge_index = torch.cat(edge_indices, dim=1)

        x = F.elu(self.conv1(x, edge_index))
        x = F.dropout(x, p=0.1, training=self.training)
        x = F.elu(self.conv2(x, edge_index))

        safety = self.safety_head(x)
        anomaly = self.anomaly_head(x)
        logits = self.class_head(x)

        # Reshape back if batched
        if 'B' in dir() and 'N' in dir():
            safety = safety.view(B, N, 1)
            anomaly = anomaly.view(B, N, 1)
            logits = logits.view(B, N, -1)

        return logits, safety
