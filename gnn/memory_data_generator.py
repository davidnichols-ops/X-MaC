#!/usr/bin/env python3
"""Memory system simulator for training the MemoryGAT model.

Generates synthetic memory graphs that model macOS unified memory architecture:
- Process nodes (24-dim features) with realistic allocation patterns
- Hardware consumer nodes (CPU, GPU, ANE — 8-dim features)
- Swap file node (6-dim features)
- Compressor pool node (6-dim features)
- Edge types: accesses, parent_of, depends_on, backs, compressed_in, contributes_to

Labels:
- System pressure trajectory (0=normal, 1=warn, 2=critical) in 60s
- Per-process action recommendation (0=no_action, 1=pressure_relief,
  2=suggest_purge, 3=deprioritize, 4=suspend, 5=terminate)
- Per-process risk score (0.0=safe to intervene, 1.0=dangerous)
- Per-process growth prediction (predicted RSS in 60s, normalized)
"""

import argparse
import json
import math
import random
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

import torch
from torch_geometric.data import Data

# ═══════════════════════════════════════════════════════════════════════
#  Constants — must match Rust telemetry/graph feature dimensions
# ═══════════════════════════════════════════════════════════════════════

PROCESS_FEATURE_DIM = 24
HARDWARE_FEATURE_DIM = 8
SWAP_FEATURE_DIM = 6
COMPRESSOR_FEATURE_DIM = 6

NUM_NODE_TYPES = 4  # process, hardware_consumer, swap_file, compressor_pool
NUM_EDGE_TYPES = 6  # accesses, parent_of, depends_on, backs, compressed_in, contributes_to

# Action labels
ACTION_LABELS = {
    0: "no_action",
    1: "pressure_relief",
    2: "suggest_purge",
    3: "deprioritize",
    4: "suspend",
    5: "terminate",
}
NUM_ACTIONS = len(ACTION_LABELS)

# Pressure labels
PRESSURE_NORMAL = 0
PRESSURE_WARN = 1
PRESSURE_CRITICAL = 2
NUM_PRESSURE = 3

PROJECT_ROOT = Path(__file__).resolve().parent.parent
GNN_DIR = PROJECT_ROOT / "gnn"
MEMORY_DATA_DIR = GNN_DIR / "memory_data"


# ═══════════════════════════════════════════════════════════════════════
#  Process archetypes — realistic macOS process profiles
# ═══════════════════════════════════════════════════════════════════════

@dataclass
class ProcessArchetype:
    """A realistic macOS process profile for simulation."""
    name: str
    base_rss_mb: float          # Typical RSS in MB
    rss_sigma: float            # Variability in RSS
    growth_rate_mb_per_min: float  # Average growth rate (can be negative)
    growth_sigma: float         # Variability in growth rate
    is_system: bool
    is_foreground: bool
    thread_count: int
    priority: int                # -20 to 20
    has_compressed: bool
    has_purgeable: bool
    has_gpu: bool
    has_neural: bool
    graphics_footprint_mb: float
    neural_footprint_mb: float
    network_footprint_mb: float
    media_footprint_mb: float
    weight: float               # Probability weight for selection


ARCHETYPES = [
    # System processes
    ProcessArchetype("kernel_task", 2048, 200, 0, 10, True, False, 200, -20,
                     False, False, False, False, 0, 0, 0, 0, 0.5),
    ProcessArchetype("launchd", 50, 10, 0, 2, True, False, 10, 0,
                     False, False, False, False, 0, 0, 0, 0, 0.3),
    ProcessArchetype("WindowServer", 256, 50, 5, 10, True, False, 30, 10,
                     True, False, True, False, 128, 0, 10, 0, 0.2),
    ProcessArchetype("Finder", 128, 30, 2, 5, True, True, 15, 0,
                     True, True, True, False, 32, 0, 5, 0, 0.3),
    ProcessArchetype("Dock", 64, 15, 1, 3, True, True, 12, 0,
                     True, True, True, False, 16, 0, 0, 0, 0.2),
    ProcessArchetype("coreaudiod", 32, 5, 0, 1, True, False, 5, 0,
                     False, False, False, False, 0, 0, 0, 5, 0.1),
    ProcessArchetype("bluetoothd", 16, 3, 0, 1, True, False, 4, 0,
                     False, False, False, False, 0, 0, 2, 0, 0.1),
    ProcessArchetype("mds_stores", 128, 40, 10, 20, True, False, 20, 0,
                     True, True, False, False, 0, 0, 0, 0, 0.2),
    ProcessArchetype("configd", 32, 5, 0, 1, True, False, 6, 0,
                     False, False, False, False, 0, 0, 5, 0, 0.1),
    ProcessArchetype("logd", 64, 20, 5, 10, True, False, 8, 0,
                     True, True, False, False, 0, 0, 0, 0, 0.1),

    # User applications — normal
    ProcessArchetype("Safari", 384, 100, 10, 30, False, True, 25, 0,
                     True, True, True, False, 64, 0, 20, 10, 0.5),
    ProcessArchetype("Mail", 256, 80, 5, 15, False, True, 15, 0,
                     True, True, False, False, 0, 0, 10, 0, 0.4),
    ProcessArchetype("Messages", 192, 50, 3, 10, False, True, 12, 0,
                     True, True, False, False, 16, 0, 5, 0, 0.4),
    ProcessArchetype("Terminal", 64, 20, 2, 5, False, True, 8, 0,
                     False, False, False, False, 0, 0, 0, 0, 0.5),
    ProcessArchetype("Xcode", 1024, 300, 20, 50, False, True, 40, 0,
                     True, True, True, False, 128, 0, 10, 0, 0.3),
    ProcessArchetype("Slack", 384, 100, 8, 20, False, True, 20, 0,
                     True, True, False, False, 32, 0, 15, 0, 0.4),
    ProcessArchetype("Spotify", 192, 50, 3, 10, False, True, 15, 0,
                     True, False, False, False, 0, 0, 5, 32, 0.3),
    ProcessArchetype("Notes", 128, 30, 2, 5, False, True, 10, 0,
                     True, True, False, False, 8, 0, 0, 0, 0.3),
    ProcessArchetype("Calendar", 96, 20, 1, 3, False, True, 8, 0,
                     True, True, False, False, 4, 0, 2, 0, 0.2),

    # User applications — heavy
    ProcessArchetype("Google Chrome", 512, 200, 15, 40, False, True, 30, 0,
                     True, True, True, False, 64, 0, 30, 10, 0.6),
    ProcessArchetype("Chrome Helper", 384, 150, 20, 60, False, False, 25, 0,
                     True, True, True, False, 48, 0, 20, 5, 0.8),
    ProcessArchetype("Chrome Renderer", 320, 120, 25, 80, False, False, 20, 0,
                     True, False, True, False, 32, 0, 15, 0, 0.7),
    ProcessArchetype("Firefox", 384, 120, 10, 30, False, True, 25, 0,
                     True, True, True, False, 48, 0, 20, 5, 0.4),
    ProcessArchetype("Docker", 768, 200, 30, 80, False, True, 35, 0,
                     True, True, False, False, 0, 0, 10, 0, 0.3),
    ProcessArchetype("node", 256, 80, 15, 40, False, False, 15, 0,
                     True, True, False, False, 0, 0, 10, 0, 0.5),
    ProcessArchetype("python", 192, 60, 10, 30, False, False, 12, 0,
                     True, True, False, False, 0, 0, 5, 0, 0.5),
    ProcessArchetype("rust-analyzer", 512, 150, 20, 50, False, False, 30, 0,
                     True, True, False, False, 0, 0, 0, 0, 0.3),

    # ML / GPU heavy
    ProcessArchetype("CoreML Worker", 256, 80, 15, 40, False, False, 15, 0,
                     True, False, True, True, 64, 32, 5, 0, 0.3),
    ProcessArchetype("MLX Server", 512, 200, 30, 80, False, False, 20, 0,
                     True, False, True, True, 128, 64, 5, 0, 0.2),
    ProcessArchetype("Stable Diffusion", 1024, 300, 50, 100, False, True, 25, 0,
                     True, False, True, True, 256, 128, 5, 0, 0.1),

    # Background daemons
    ProcessArchetype("backupd", 547, 100, 50, 200, False, False, 10, 0,
                     True, True, False, False, 0, 0, 10, 0, 0.2),
    ProcessArchetype("photoanalysisd", 256, 80, 20, 60, False, False, 12, 0,
                     True, True, False, True, 32, 16, 5, 0, 0.2),
    ProcessArchetype("cloudd", 128, 40, 10, 30, False, False, 8, 0,
                     True, True, False, False, 0, 0, 20, 0, 0.2),
    ProcessArchetype("sharingd", 64, 20, 2, 5, False, False, 6, 0,
                     False, False, False, False, 0, 0, 10, 0, 0.2),

    # Leaky / problematic (for training the model to identify issues)
    ProcessArchetype("LeakyApp", 800, 200, 100, 50, False, True, 20, 0,
                     True, False, False, False, 0, 0, 10, 0, 0.1),
    ProcessArchetype("CacheBloat", 600, 150, 30, 20, False, False, 15, 0,
                     True, True, False, False, 0, 0, 5, 0, 0.1),
    ProcessArchetype("ZombieProc", 50, 10, -5, 5, False, False, 2, 20,
                     False, False, False, False, 0, 0, 0, 0, 0.05),
]


# ═══════════════════════════════════════════════════════════════════════
#  Memory System Simulator
# ═══════════════════════════════════════════════════════════════════════

class MemorySystemSimulator:
    """Simulates a macOS system with processes, memory regions, and pressure."""

    def __init__(self, total_memory_mb=16384, rng=None):
        self.total_memory_mb = total_memory_mb
        self.rng = rng or random.Random()
        self.page_size = 4096

    def simulate_scenario(self, scenario: str = "random") -> dict:
        """Simulate a specific memory pressure scenario.

        Returns a dict with:
        - processes: list of process dicts
        - system: system telemetry dict
        - labels: {pressure_60s, process_actions, process_risks, process_growth}
        """
        if scenario == "random":
            scenario = self.rng.choice([
                "healthy", "memory_leak", "cache_bloat", "gpu_pressure",
                "swap_thrashing", "jetsam_cascade", "startup_burst",
                "idle_background", "mixed_workload",
            ])

        # Select processes for this scenario
        num_processes = self.rng.randint(20, 80)
        processes = self._select_processes(num_processes, scenario)

        # Compute system telemetry from processes
        system = self._compute_system_telemetry(processes, scenario)

        # Compute labels
        labels = self._compute_labels(processes, system, scenario)

        return {
            "processes": processes,
            "system": system,
            "labels": labels,
            "scenario": scenario,
        }

    def _select_processes(self, n: int, scenario: str) -> list:
        """Select n processes weighted by archetype weights, adjusted for scenario."""
        # Adjust weights based on scenario
        weights = [a.weight for a in ARCHETYPES]

        if scenario == "memory_leak":
            # Boost leaky apps
            weights = [
                w * 5.0 if a.name == "LeakyApp" else w
                for a, w in zip(ARCHETYPES, weights)
            ]
        elif scenario == "cache_bloat":
            weights = [
                w * 5.0 if a.name == "CacheBloat" else w
                for a, w in zip(ARCHETYPES, weights)
            ]
        elif scenario == "gpu_pressure":
            weights = [
                w * 5.0 if a.has_gpu or a.has_neural else w
                for a, w in zip(ARCHETYPES, weights)
            ]
        elif scenario == "startup_burst":
            # More processes, more heavy ones
            n = min(n + 20, 100)
            weights = [
                w * 2.0 if a.base_rss_mb > 500 else w
                for a, w in zip(ARCHETYPES, weights)
            ]
        elif scenario == "idle_background":
            # Fewer processes, mostly background
            n = min(n, 40)
            weights = [
                w * 3.0 if not a.is_foreground else w * 0.3
                for a, w in zip(ARCHETYPES, weights)
            ]

        # Normalize weights
        total = sum(weights)
        weights = [w / total for w in weights]

        selected = self.rng.choices(ARCHETYPES, weights=weights, k=n)

        # Build process instances
        processes = []
        pid_counter = 1
        for archetype in selected:
            # Add variability to RSS
            rss_mb = max(
                1,
                self.rng.gauss(archetype.base_rss_mb, archetype.rss_sigma),
            )
            # Growth rate (MB/min)
            growth_rate = self.rng.gauss(
                archetype.growth_rate_mb_per_min, archetype.growth_sigma
            )
            # For leaky apps, ensure positive growth
            if archetype.name == "LeakyApp":
                growth_rate = abs(growth_rate) + 50

            virtual_mb = rss_mb * self.rng.uniform(50, 500)
            compressed_mb = rss_mb * self.rng.uniform(0.1, 0.3) if archetype.has_compressed else 0
            internal_mb = rss_mb * self.rng.uniform(0.3, 0.6)
            external_mb = rss_mb * self.rng.uniform(0.1, 0.3)
            reusable_mb = rss_mb * self.rng.uniform(0.05, 0.2) if archetype.has_purgeable else 0
            purgeable_mb = rss_mb * self.rng.uniform(0.05, 0.15) if archetype.has_purgeable else 0

            processes.append({
                "pid": pid_counter,
                "ppid": 0,  # will be set later
                "name": archetype.name,
                "rss_mb": rss_mb,
                "virtual_mb": virtual_mb,
                "compressed_mb": compressed_mb,
                "internal_mb": internal_mb,
                "external_mb": external_mb,
                "reusable_mb": reusable_mb,
                "purgeable_mb": purgeable_mb,
                "graphics_mb": archetype.graphics_footprint_mb * self.rng.uniform(0.5, 1.5),
                "neural_mb": archetype.neural_footprint_mb * self.rng.uniform(0.5, 1.5),
                "network_mb": archetype.network_footprint_mb * self.rng.uniform(0.5, 1.5),
                "media_mb": archetype.media_footprint_mb * self.rng.uniform(0.5, 1.5),
                "growth_rate_mb_per_min": growth_rate,
                "thread_count": archetype.thread_count + self.rng.randint(-2, 5),
                "priority": archetype.priority,
                "is_system": archetype.is_system,
                "is_foreground": archetype.is_foreground,
                "page_faults": int(self.rng.uniform(100, 10000)),
                "pageins": int(self.rng.uniform(0, 1000)),
                "cow_faults": int(self.rng.uniform(10, 500)),
            })
            pid_counter += 1

        # Assign parent-child relationships
        self._assign_parents(processes)

        return processes

    def _assign_parents(self, processes: list):
        """Assign parent PIDs to create a realistic process tree."""
        system_procs = [p for p in processes if p["is_system"]]
        user_procs = [p for p in processes if not p["is_system"]]

        # launchd (pid 1) is the root
        if system_procs:
            system_procs[0]["ppid"] = 0  # launchd's parent is kernel

        # System processes are children of launchd or other system procs
        for i, proc in enumerate(system_procs[1:], 1):
            # 70% chance parent is launchd, 30% chance another system proc
            if self.rng.random() < 0.7 and system_procs:
                proc["ppid"] = system_procs[0]["pid"]
            else:
                proc["ppid"] = self.rng.choice(system_procs[:max(i, 1)])["pid"]

        # User processes are children of launchd or a shell
        for proc in user_procs:
            if self.rng.random() < 0.5 and system_procs:
                proc["ppid"] = system_procs[0]["pid"]
            elif user_procs:
                # Parent is another user process (e.g., Terminal → bash → node)
                proc["ppid"] = self.rng.choice(user_procs)["pid"]
            else:
                proc["ppid"] = 1

    def _compute_system_telemetry(self, processes: list, scenario: str) -> dict:
        """Compute system-wide telemetry from process list."""
        total_rss = sum(p["rss_mb"] for p in processes)
        total_compressed = sum(p["compressed_mb"] for p in processes)
        total_graphics = sum(p["graphics_mb"] for p in processes)
        total_neural = sum(p["neural_mb"] for p in processes)

        # Wired memory: ~10-15% of total on macOS
        wired_mb = self.total_memory_mb * self.rng.uniform(0.10, 0.15)

        # Active memory: NOT the sum of all RSS (processes share pages).
        # On a real Mac, active is typically 30-50% of total.
        # Use a fraction of total RSS, capped at 50% of total.
        active_mb = min(total_rss * 0.4, self.total_memory_mb * 0.45)

        # Inactive: pages not recently accessed — 5-15% of total
        inactive_mb = self.total_memory_mb * self.rng.uniform(0.05, 0.15)

        # Free memory: total - wired - active - compressed - inactive
        # On a healthy 16GB Mac, free is typically 4-8 GB (25-50%)
        used = wired_mb + active_mb + total_compressed + inactive_mb
        free_mb = max(0, self.total_memory_mb - used)

        # For healthy scenario, boost free memory
        if scenario == "healthy":
            free_mb = max(free_mb, self.total_memory_mb * self.rng.uniform(0.25, 0.50))
            active_mb = min(active_mb, self.total_memory_mb * 0.30)
        elif scenario == "idle_background":
            free_mb = max(free_mb, self.total_memory_mb * self.rng.uniform(0.30, 0.55))
            active_mb = min(active_mb, self.total_memory_mb * 0.25)

        # Speculative
        speculative_mb = self.total_memory_mb * self.rng.uniform(0.01, 0.03)

        # Purgeable
        purgeable_mb = sum(p["purgeable_mb"] for p in processes)

        # Compressor
        compressor_pool_mb = self.total_memory_mb * 0.25
        compressor_used_mb = total_compressed

        # Swap
        swap_total_mb = self.total_memory_mb * 2
        if scenario == "swap_thrashing":
            swap_used_mb = self.rng.uniform(2000, 8000)
        elif free_mb < self.total_memory_mb * 0.05:
            swap_used_mb = self.rng.uniform(500, 3000)
        else:
            swap_used_mb = self.rng.uniform(0, 500)

        # Pressure level
        utilization = used / self.total_memory_mb
        if utilization > 0.90:
            pressure_level = 4  # critical
        elif utilization > 0.75:
            pressure_level = 2  # warn
        else:
            pressure_level = 1  # normal

        # GPU working set limit (~66-75% of total RAM)
        gpu_limit_mb = self.total_memory_mb * 0.70
        gpu_wired_mb = total_graphics + total_neural

        return {
            "total_mb": self.total_memory_mb,
            "page_size": self.page_size,
            "free_mb": free_mb,
            "active_mb": active_mb,
            "inactive_mb": inactive_mb,
            "speculative_mb": speculative_mb,
            "wired_mb": wired_mb,
            "purgeable_mb": purgeable_mb,
            "compressor_used_mb": compressor_used_mb,
            "compressor_pool_mb": compressor_pool_mb,
            "swap_total_mb": swap_total_mb,
            "swap_used_mb": swap_used_mb,
            "pressure_level": pressure_level,
            "utilization": utilization,
            "gpu_limit_mb": gpu_limit_mb,
            "gpu_wired_mb": gpu_wired_mb,
            "compressions": int(total_compressed / 4),  # approx pages
            "decompressions": int(total_compressed / 8),
            "swapins": int(swap_used_mb / 4),
            "swapouts": int(swap_used_mb / 8),
            "pageins": int(active_mb / 4),
            "pageouts": int(swap_used_mb / 4),
            "faults": sum(p["page_faults"] for p in processes),
            "cow_faults": sum(p["cow_faults"] for p in processes),
            "reactivations": int(inactive_mb / 4),
        }

    def _compute_labels(self, processes: list, system: dict, scenario: str) -> dict:
        """Compute training labels based on the simulated state.

        Labels:
        - pressure_60s: predicted pressure in 60s (0=normal, 1=warn, 2=critical)
        - process_actions: per-process optimal action (0-5)
        - process_risks: per-process risk score (0.0-1.0)
        - process_growth: per-process predicted RSS in 60s (MB)
        """
        total = system["total_mb"]
        current_free = system["free_mb"]

        # Predict free memory in 60s based on process growth rates
        # Most processes are stable or shrinking. Only a few grow.
        # Model: 70% of processes have ~0 net growth, 20% shrink, 10% grow
        total_growth_per_min = 0
        for p in processes:
            rate = p["growth_rate_mb_per_min"]
            # Apply per-process damping: most processes don't grow continuously
            if rate > 0:
                # Only 20% chance the process is actively growing this minute
                if self.rng.random() < 0.2:
                    total_growth_per_min += rate
            else:
                # Negative growth (freeing) — 50% chance active
                if self.rng.random() < 0.5:
                    total_growth_per_min += rate
        predicted_free_60s = current_free - total_growth_per_min
        predicted_utilization = 1.0 - (predicted_free_60s / total)

        if predicted_utilization > 0.90:
            pressure_60s = PRESSURE_CRITICAL
        elif predicted_utilization > 0.75:
            pressure_60s = PRESSURE_WARN
        else:
            pressure_60s = PRESSURE_NORMAL

        # Per-process labels
        process_actions = []
        process_risks = []
        process_growth = []

        for p in processes:
            # Predicted RSS in 60s
            growth_60s = p["growth_rate_mb_per_min"]  # MB per minute → 60s
            predicted_rss = p["rss_mb"] + growth_60s
            process_growth.append(max(0, predicted_rss))

            # Risk score: how dangerous is it to intervene?
            risk = 0.0
            if p["is_system"]:
                risk += 0.8
            if p["is_foreground"]:
                risk += 0.4
            if p["name"] in ("kernel_task", "launchd", "WindowServer", "Finder", "Dock"):
                risk += 0.2
            risk = min(1.0, risk)
            process_risks.append(risk)

            # Optimal action
            if risk > 0.7:
                action = 0  # no_action — too risky
            elif p["purgeable_mb"] > p["rss_mb"] * 0.1:
                action = 2  # suggest_purge — has purgeable memory
            elif p["compressed_mb"] > p["rss_mb"] * 0.2:
                action = 1  # pressure_relief — has compressed memory
            elif not p["is_foreground"] and p["growth_rate_mb_per_min"] > 50:
                action = 5  # terminate — leaky background process
            elif not p["is_foreground"] and p["rss_mb"] > 500:
                action = 4  # suspend — large background process
            elif not p["is_foreground"] and p["priority"] > 10:
                action = 3  # deprioritize — low priority background
            elif p["reusable_mb"] > p["rss_mb"] * 0.1:
                action = 1  # pressure_relief — has reusable memory
            else:
                action = 0  # no_action

            # Override for specific scenarios
            if scenario == "memory_leak" and p["name"] == "LeakyApp":
                if risk < 0.5:
                    action = 5  # terminate the leak
            elif scenario == "cache_bloat" and p["name"] == "CacheBloat":
                action = 2  # suggest_purge

            process_actions.append(action)

        return {
            "pressure_60s": pressure_60s,
            "process_actions": process_actions,
            "process_risks": process_risks,
            "process_growth": process_growth,
        }


# ═══════════════════════════════════════════════════════════════════════
#  Graph Construction (convert simulation to PyG Data)
# ═══════════════════════════════════════════════════════════════════════

def simulation_to_graph(sim: dict, rng=None) -> Data:
    """Convert a simulation result to a PyTorch Geometric Data object.

    Node types: 0=process, 1=hardware_consumer, 2=swap_file, 3=compressor_pool
    Edge types: 0=accesses, 1=parent_of, 2=depends_on, 3=backs,
                4=compressed_in, 5=contributes_to
    """
    rng = rng or random.Random()
    processes = sim["processes"]
    system = sim["system"]
    labels = sim["labels"]
    total_mb = system["total_mb"]

    nodes_x = []
    node_types = []
    edge_index = [[], []]
    edge_types = []

    # ── Process nodes (24-dim features) ─────────────────────────────
    process_node_ids = []
    for i, p in enumerate(processes):
        node_id = i
        process_node_ids.append(node_id)

        rss = p["rss_mb"]
        virtual = p["virtual_mb"]
        phys_footprint = rss  # proxy
        compressed = p["compressed_mb"]
        internal = p["internal_mb"]
        external = p["external_mb"]
        reusable = p["reusable_mb"]
        purgeable = p["purgeable_mb"]
        graphics = p["graphics_mb"]
        neural = p["neural_mb"]
        network = p["network_mb"]
        media = p["media_mb"]
        growth_rate = p["growth_rate_mb_per_min"]
        page_faults = p["page_faults"]
        pageins = p["pageins"]
        cow_faults = p["cow_faults"]
        thread_count = p["thread_count"]
        priority = p["priority"]
        is_foreground = p["is_foreground"]
        is_system = p["is_system"]
        pid = p["pid"]

        # Normalize features (matching Rust graph.rs extract_process_features)
        features = [
            math.log1p(rss * 1024 * 1024) / 20.0,           # 0: log(RSS)
            rss / total_mb,                                    # 1: RSS / total
            virtual / total_mb,                               # 2: virtual / total
            phys_footprint / total_mb,                         # 3: phys_footprint / total
            (compressed / rss) if rss > 0 else 0.0,            # 4: compressed / RSS
            (internal / rss) if rss > 0 else 0.0,              # 5: internal / RSS
            (external / rss) if rss > 0 else 0.0,              # 6: external / RSS
            (reusable / rss) if rss > 0 else 0.0,              # 7: reusable / RSS
            (purgeable / rss) if rss > 0 else 0.0,             # 8: purgeable / RSS
            graphics / total_mb,                               # 9: graphics / total
            neural / total_mb,                                # 10: neural / total
            network / total_mb,                               # 11: network / total
            media / total_mb,                                 # 12: media / total
            math.tanh(growth_rate / (total_mb * 0.01)),       # 13: growth rate (normalized)
            math.log1p(page_faults) / 15.0,                   # 14: page faults
            math.log1p(pageins) / 15.0,                       # 15: pageins
            math.log1p(cow_faults) / 15.0,                     # 16: cow faults
            math.log1p(thread_count) / 8.0,                    # 17: thread count
            (priority + 20) / 40.0,                           # 18: priority
            1.0 if is_foreground else 0.0,                    # 19: is_foreground
            1.0 if is_system else 0.0,                        # 20: is_system
            pid / 100000.0,                                   # 21: pid
            1.0 if compressed > 0 else 0.0,                   # 22: has compressed
            1.0 if purgeable > 0 else 0.0,                    # 23: has purgeable
        ]
        nodes_x.append(features)
        node_types.append(0)  # process

    num_processes = len(processes)

    # ── Hardware consumer nodes (8-dim features) ────────────────────
    # CPU
    cpu_id = num_processes
    nodes_x.append(_cpu_features(system))
    node_types.append(1)  # hardware_consumer

    # GPU
    gpu_id = num_processes + 1
    nodes_x.append(_gpu_features(system))
    node_types.append(1)

    # ANE
    ane_id = num_processes + 2
    nodes_x.append(_ane_features(system))
    node_types.append(1)

    # ── Swap file node (6-dim) ──────────────────────────────────────
    swap_id = num_processes + 3
    nodes_x.append(_swap_features(system))
    node_types.append(2)  # swap_file

    # ── Compressor pool node (6-dim) ────────────────────────────────
    compressor_id = num_processes + 4
    nodes_x.append(_compressor_features(system))
    node_types.append(3)  # compressor_pool

    # ── Edges ───────────────────────────────────────────────────────

    def add_edge(src, tgt, etype):
        edge_index[0].append(src)
        edge_index[1].append(tgt)
        edge_types.append(etype)

    # Process → CPU (accesses = 0)
    for pid in process_node_ids:
        add_edge(pid, cpu_id, 0)

    # Process → GPU (if has graphics footprint)
    for i, p in enumerate(processes):
        if p["graphics_mb"] > 0:
            add_edge(i, gpu_id, 0)

    # Process → ANE (if has neural footprint)
    for i, p in enumerate(processes):
        if p["neural_mb"] > 0:
            add_edge(i, ane_id, 0)

    # Process → Process (parent_of = 1)
    pid_to_node = {p["pid"]: i for i, p in enumerate(processes)}
    for i, p in enumerate(processes):
        ppid = p["ppid"]
        if ppid in pid_to_node and pid_to_node[ppid] != i:
            add_edge(pid_to_node[ppid], i, 1)

    # Process → Process (depends_on = 2) — siblings
    siblings = {}
    for p in processes:
        siblings.setdefault(p["ppid"], []).append(pid_to_node[p["pid"]])
    for group in siblings.values():
        if 1 < len(group) <= 5:
            for i in range(len(group)):
                for j in range(i + 1, len(group)):
                    add_edge(group[i], group[j], 2)

    # CPU → Compressor (contributes_to = 5)
    add_edge(cpu_id, compressor_id, 5)

    # Compressor → Swap (backs = 3)
    if system["swap_used_mb"] > 0:
        add_edge(compressor_id, swap_id, 3)

    # Process → Compressor (compressed_in = 4)
    for i, p in enumerate(processes):
        if p["compressed_mb"] > 0:
            add_edge(i, compressor_id, 4)

    # ── Labels ──────────────────────────────────────────────────────
    # Per-process labels (only for process nodes, pad others)
    num_nodes = len(nodes_x)
    action_labels = [0] * num_nodes
    risk_labels = [0.0] * num_nodes
    growth_labels = [0.0] * num_nodes

    for i, (action, risk, growth) in enumerate(zip(
        labels["process_actions"],
        labels["process_risks"],
        labels["process_growth"],
    )):
        action_labels[i] = action
        risk_labels[i] = risk
        growth_labels[i] = growth / total_mb  # normalize

    # System-level pressure label
    pressure_label = labels["pressure_60s"]

    # ── Build PyG Data ──────────────────────────────────────────────
    # Pad all node features to MAX_FEATURE_DIM (24) for uniform tensor
    padded_x = []
    for features in nodes_x:
        if len(features) < PROCESS_FEATURE_DIM:
            features = features + [0.0] * (PROCESS_FEATURE_DIM - len(features))
        padded_x.append(features)

    x = torch.tensor(padded_x, dtype=torch.float32)
    edge_index = torch.tensor(edge_index, dtype=torch.long)
    edge_type = torch.tensor(edge_types, dtype=torch.long)
    node_type = torch.tensor(node_types, dtype=torch.long)

    data = Data(x=x, edge_index=edge_index, edge_type=edge_type)
    data.node_type = node_type
    data.action_labels = torch.tensor(action_labels, dtype=torch.long)
    data.risk_labels = torch.tensor(risk_labels, dtype=torch.float32)
    data.growth_labels = torch.tensor(growth_labels, dtype=torch.float32)
    data.pressure_label = torch.tensor([pressure_label], dtype=torch.long)
    data.num_processes = num_processes

    return data


def _cpu_features(system: dict) -> list:
    total = system["total_mb"]
    return [
        system["wired_mb"] / total,                        # 0: wired / total
        system["active_mb"] / total,                        # 1: active / total
        system["free_mb"] / total,                          # 2: free / total
        system["utilization"],                              # 3: utilization
        {1: 0.0, 2: 0.5, 4: 1.0}[system["pressure_level"]],  # 4: pressure
        math.log1p(system["faults"]) / 20.0,                # 5: faults
        math.log1p(system["cow_faults"]) / 20.0,            # 6: cow faults
        0.0,                                                # 7: free rate (not tracked in sim)
    ]


def _gpu_features(system: dict) -> list:
    total = system["total_mb"]
    gpu_limit = system["gpu_limit_mb"]
    return [
        gpu_limit / total,                                  # 0: GPU limit / total
        system["gpu_wired_mb"] / total,                     # 1: GPU wired / total
        (system["gpu_wired_mb"] / gpu_limit) if gpu_limit > 0 else 0.0,  # 2: GPU wired / limit
        1.0 if gpu_limit > 0 else 0.0,                      # 3: has GPU limit
        system["speculative_mb"] / total,                   # 4: speculative / total
        system["purgeable_mb"] / total,                     # 5: purgeable / total
        {1: 0.0, 2: 0.5, 4: 1.0}[system["pressure_level"]],  # 6: pressure
        system["utilization"],                              # 7: utilization
    ]


def _ane_features(system: dict) -> list:
    total = system["total_mb"]
    return [
        0.0,                                                # 0: ANE proxy
        1.0,                                                # 1: reference
        {1: 0.0, 2: 0.5, 4: 1.0}[system["pressure_level"]],  # 2: pressure
        system["utilization"],                              # 3: utilization
        system["free_mb"] / total,                          # 4: free / total
        system["wired_mb"] / total,                         # 5: wired / total
        system["compressor_used_mb"] / total,               # 6: compressor / total
        system["swap_used_mb"] / total,                     # 7: swap / total
    ]


def _swap_features(system: dict) -> list:
    total = system["swap_total_mb"] or 1
    return [
        system["swap_used_mb"] / total,                     # 0: used / total
        system["swap_total_mb"] / system["total_mb"],       # 1: swap total / system total
        math.log1p(system["swapins"]) / 15.0,               # 2: swapins
        math.log1p(system["swapouts"]) / 15.0,              # 3: swapouts
        1.0 if system["swap_total_mb"] > 0 else 0.0,       # 4: has swap
        system["swap_used_mb"] / total,                     # 5: swap pressure
    ]


def _compressor_features(system: dict) -> list:
    total = system["total_mb"]
    pool = system["compressor_pool_mb"] or 1
    return [
        system["compressor_used_mb"] / pool,                # 0: used / pool
        system["compressor_used_mb"] / total,               # 1: used / total
        (system["compressor_used_mb"] / system["compressions"]) if system["compressions"] > 0 else 1.0,  # 2: ratio
        math.log1p(system["compressions"]) / 20.0,         # 3: compressions
        math.log1p(system["decompressions"]) / 20.0,       # 4: decompressions
        system["compressor_pool_mb"] / total,               # 5: pool / total
    ]


# ═══════════════════════════════════════════════════════════════════════
#  Dataset Generation
# ═══════════════════════════════════════════════════════════════════════

def generate_dataset(
    num_graphs: int = 10000,
    train_ratio: float = 0.70,
    val_ratio: float = 0.15,
    test_ratio: float = 0.15,
    seed: int = 42,
    output_dir: Optional[Path] = None,
):
    """Generate a dataset of simulated memory graphs."""
    if output_dir is None:
        output_dir = MEMORY_DATA_DIR
    output_dir.mkdir(parents=True, exist_ok=True)

    rng = random.Random(seed)
    simulator = MemorySystemSimulator(rng=rng)

    scenarios = [
        "healthy", "memory_leak", "cache_bloat", "gpu_pressure",
        "swap_thrashing", "jetsam_cascade", "startup_burst",
        "idle_background", "mixed_workload", "random",
    ]

    graphs = []
    for i in range(num_graphs):
        scenario = rng.choice(scenarios)
        sim = simulator.simulate_scenario(scenario)
        data = simulation_to_graph(sim, rng=rng)
        graphs.append(data)

    # Split
    rng.shuffle(graphs)
    n_train = int(num_graphs * train_ratio)
    n_val = int(num_graphs * val_ratio)
    train_graphs = graphs[:n_train]
    val_graphs = graphs[n_train:n_train + n_val]
    test_graphs = graphs[n_train + n_val:]

    # Save
    torch.save(train_graphs, output_dir / "train.pt")
    torch.save(val_graphs, output_dir / "val.pt")
    torch.save(test_graphs, output_dir / "test.pt")

    # Statistics
    action_dist = Counter()
    pressure_dist = Counter()
    total_action_labels = 0
    for g in graphs:
        for a in g.action_labels[:g.num_processes].tolist():
            action_dist[ ACTION_LABELS[a] ] += 1
            total_action_labels += 1
        pressure_dist[g.pressure_label.item()] += 1

    stats = {
        "total_graphs": num_graphs,
        "train": len(train_graphs),
        "val": len(val_graphs),
        "test": len(test_graphs),
        "action_distribution": dict(action_dist),
        "pressure_distribution": {
            f"{'normal' if k == 0 else 'warn' if k == 1 else 'critical'}": v
            for k, v in sorted(pressure_dist.items())
        },
        "scenarios": scenarios,
        "feature_dims": {
            "process": PROCESS_FEATURE_DIM,
            "hardware": HARDWARE_FEATURE_DIM,
            "swap": SWAP_FEATURE_DIM,
            "compressor": COMPRESSOR_FEATURE_DIM,
        },
    }

    with open(output_dir / "dataset_stats.json", "w") as f:
        json.dump(stats, f, indent=2)

    print(f"Generated {num_graphs} memory graphs:")
    print(f"  Train: {len(train_graphs)}")
    print(f"  Val:   {len(val_graphs)}")
    print(f"  Test:  {len(test_graphs)}")
    print(f"\nAction distribution ({total_action_labels} total labels):")
    for action, count in sorted(action_dist.items(), key=lambda x: -x[1]):
        print(f"  {action:20s} {count:6d} ({count/total_action_labels*100:.1f}%)")
    print(f"\nPressure distribution:")
    for level, count in sorted(pressure_dist.items()):
        label = "normal" if level == 0 else "warn" if level == 1 else "critical"
        print(f"  {label:20s} {count:6d} ({count/num_graphs*100:.1f}%)")

    return stats


# ═══════════════════════════════════════════════════════════════════════
#  Main
# ═══════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generate synthetic memory system graphs for GNN training"
    )
    parser.add_argument(
        "--num-graphs", type=int, default=10000,
        help="Number of graphs to generate (default: 10000)"
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed for reproducibility (default: 42)"
    )
    parser.add_argument(
        "--output-dir", type=Path, default=MEMORY_DATA_DIR,
        help="Output directory for generated data"
    )
    args = parser.parse_args()

    generate_dataset(
        num_graphs=args.num_graphs,
        seed=args.seed,
        output_dir=args.output_dir,
    )
