#!/usr/bin/env python3

import argparse
import json
import math
import random
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

import torch
from torch_geometric.data import Data

NUM_FEATURES = 16
MIN_NODES = 50
MAX_NODES = 200
PROJECT_ROOT = Path(__file__).resolve().parent.parent
GNN_DIR = PROJECT_ROOT / "gnn"
DATA_DIR = GNN_DIR / "data"
LABEL_MAP_PATH = GNN_DIR / "label_map.json"

with LABEL_MAP_PATH.open() as label_file:
    LABEL_MAP = json.load(label_file)
IDX_TO_LABEL = {index: label for label, index in LABEL_MAP.items()}


@dataclass(frozen=True)
class CategorySpec:
    safety: float
    anomaly: float
    size_log_mean: float
    size_log_sigma: float
    age_mean: float
    access_mean: float
    is_dir: bool = False
    extension: str | None = None
    hidden: bool = False
    executable: bool = False
    weight: float = 1.0
    graph_probability: float = 0.35


CATEGORY_SPECS = {
    "app_bundle": CategorySpec(0.12, 0.45, 0.66, 0.012, 0.30, 0.12, True, "app", False, True, 0.4, 0.25),
    "archive": CategorySpec(0.55, 0.35, 0.58, 0.012, 0.52, 0.38, False, "zip", False, False, 0.8, 0.55),
    "audio": CategorySpec(0.35, 0.30, 0.50, 0.010, 0.38, 0.22, False, "mp3", False, False, 0.6, 0.45),
    "backup_dir": CategorySpec(0.50, 0.45, 0.74, 0.010, 0.70, 0.58, True, None, False, False, 0.25, 0.08),
    "build_output": CategorySpec(0.85, 0.20, 0.36, 0.008, 0.13, 0.04, True, None, False, False, 1.4, 0.82),
    "cache_dir": CategorySpec(0.92, 0.15, 0.29, 0.008, 0.18, 0.02, True, None, False, False, 1.6, 0.88),
    "cache_file": CategorySpec(0.90, 0.10, 0.25, 0.008, 0.16, 0.01, False, "cache", False, False, 2.3, 0.90),
    "cargo_target": CategorySpec(0.88, 0.15, 0.40, 0.008, 0.11, 0.03, True, None, False, False, 1.0, 0.72),
    "config_file": CategorySpec(0.10, 0.50, 0.12, 0.007, 0.34, 0.06, False, "json", False, False, 1.9, 0.88),
    "directory": CategorySpec(0.30, 0.25, 0.02, 0.004, 0.42, 0.08, True, None, False, False, 3.0, 1.0),
    "disk_image": CategorySpec(0.60, 0.40, 0.82, 0.008, 0.47, 0.40, False, "dmg", False, False, 0.25, 0.08),
    "document": CategorySpec(0.25, 0.35, 0.32, 0.008, 0.44, 0.19, False, "pdf", False, False, 1.2, 0.72),
    "executable": CategorySpec(0.15, 0.55, 0.44, 0.008, 0.28, 0.08, False, None, False, True, 0.8, 0.58),
    "file": CategorySpec(0.30, 0.30, 0.18, 0.007, 0.40, 0.16, False, None, False, False, 3.0, 1.0),
    "git_dir": CategorySpec(0.08, 0.55, 0.22, 0.007, 0.26, 0.04, True, None, True, False, 1.0, 0.82),
    "image": CategorySpec(0.40, 0.30, 0.42, 0.008, 0.31, 0.13, False, "png", False, False, 1.0, 0.68),
    "language_file": CategorySpec(0.70, 0.20, 0.09, 0.006, 0.48, 0.20, False, "lproj", False, False, 0.45, 0.32),
    "library_dir": CategorySpec(0.15, 0.40, 0.46, 0.008, 0.62, 0.15, True, None, False, False, 0.7, 0.50),
    "library_file": CategorySpec(0.20, 0.50, 0.54, 0.008, 0.60, 0.14, False, "dylib", False, False, 0.9, 0.62),
    "log_dir": CategorySpec(0.82, 0.25, 0.20, 0.007, 0.57, 0.32, True, None, False, False, 0.9, 0.64),
    "log_file": CategorySpec(0.80, 0.30, 0.27, 0.008, 0.55, 0.30, False, "log", False, False, 1.6, 0.84),
    "object_file": CategorySpec(0.84, 0.22, 0.34, 0.007, 0.10, 0.02, False, "o", False, False, 1.2, 0.76),
    "python_cache": CategorySpec(0.90, 0.10, 0.15, 0.006, 0.08, 0.01, True, None, True, False, 1.1, 0.74),
    "root": CategorySpec(0.00, 0.50, 0.00, 0.000, 0.00, 0.00, True, None, False, False, 0.0, 1.0),
    "source_code": CategorySpec(0.05, 0.60, 0.07, 0.006, 0.24, 0.03, False, "rs", False, False, 2.4, 0.92),
    "trash": CategorySpec(0.95, 0.05, 0.62, 0.010, 0.86, 0.72, True, None, True, False, 0.25, 0.08),
    "video": CategorySpec(0.45, 0.35, 0.90, 0.008, 0.36, 0.24, False, "mp4", False, False, 0.5, 0.38),
}

ARCHIVE_EXTENSIONS = {"zip", "gz", "tar", "dmg", "iso", "rar", "7z"}
CODE_EXTENSIONS = {"rs", "swift", "py", "js", "ts", "c", "cpp", "h", "go", "rb", "java"}
MEDIA_EXTENSIONS = {"png", "jpg", "jpeg", "mp4", "mov", "mp3", "aac", "pdf"}
CONFIG_EXTENSIONS = {"json", "yaml", "yml", "toml", "xml", "plist", "conf", "ini", "env"}
EXTENSION_CHOICES = {
    "archive": ("zip", "gz", "tar", "rar", "7z"),
    "audio": ("mp3", "aac"),
    "config_file": ("json", "yaml", "yml", "toml", "xml", "plist", "conf", "ini", "env"),
    "disk_image": ("dmg", "iso"),
    "document": ("pdf",),
    "image": ("png", "jpg", "jpeg"),
    "source_code": ("rs", "swift", "py", "js", "ts", "c", "cpp", "h", "go", "rb", "java"),
    "video": ("mp4", "mov"),
}
PARENT_CATEGORIES = {
    "app_bundle": ("directory",),
    "archive": ("directory", "backup_dir"),
    "audio": ("directory",),
    "backup_dir": ("directory",),
    "build_output": ("directory", "cargo_target"),
    "cache_dir": ("directory", "cache_dir"),
    "cache_file": ("cache_dir",),
    "cargo_target": ("directory",),
    "config_file": ("directory",),
    "directory": ("root", "directory"),
    "disk_image": ("directory",),
    "document": ("directory",),
    "executable": ("build_output", "cargo_target", "directory"),
    "file": ("directory", "trash"),
    "git_dir": ("directory",),
    "image": ("directory",),
    "language_file": ("app_bundle", "directory"),
    "library_dir": ("root", "directory"),
    "library_file": ("library_dir",),
    "log_dir": ("root", "directory"),
    "log_file": ("log_dir",),
    "object_file": ("build_output", "cargo_target"),
    "python_cache": ("directory",),
    "source_code": ("directory",),
    "trash": ("directory",),
    "video": ("directory",),
}
NAME_STEMS = {
    "root": "root", "app_bundle": "Xcode", "archive": "release", "audio": "recording", "backup_dir": "Backups",
    "build_output": "build", "cache_dir": "Caches", "cache_file": "entry", "cargo_target": "target",
    "config_file": "config", "directory": "folder", "disk_image": "installer", "document": "document",
    "executable": "xmac", "file": "data", "git_dir": ".git", "image": "photo", "language_file": "Localizable",
    "library_dir": "lib", "library_file": "libxmac", "log_dir": "Logs", "log_file": "service",
    "object_file": "module", "python_cache": "__pycache__", "source_code": "main", "trash": ".Trash",
    "video": "movie",
}


def make_node_features(size_log, depth, max_depth, is_dir, is_file, is_symlink, is_hidden,
                       age_norm, access_norm, extension, num_children, is_executable, parent_is_dir):
    ext = extension.lower() if extension else ""
    return [
        min(max(size_log, 0.0), 1.0),
        min(depth / max(max_depth, 1), 1.0),
        float(is_dir),
        float(is_file),
        float(is_symlink),
        float(is_hidden),
        min(max(age_norm, 0.0), 1.0),
        min(max(access_norm, 0.0), 1.0),
        float(bool(extension)),
        min(num_children / 50.0, 1.0),
        float(is_executable),
        float(ext in ARCHIVE_EXTENSIONS),
        float(ext in CODE_EXTENSIONS),
        float(ext in MEDIA_EXTENSIONS),
        float(ext in CONFIG_EXTENSIONS),
        float(parent_is_dir),
    ]


class TreeBuilder:
    def __init__(self, rng):
        self.rng = rng
        self.nodes = []
        self.children = []
        self.edges = []

    def add(self, category, parent=None):
        spec = CATEGORY_SPECS[category]
        extension = self.rng.choice(EXTENSION_CHOICES.get(category, (spec.extension,)))
        suffix = f".{extension}" if extension else ""
        name = f"{NAME_STEMS[category]}-{len(self.nodes)}{suffix}"
        if category in {"git_dir", "python_cache", "trash"}:
            name = NAME_STEMS[category]
        if category == "app_bundle":
            name = f"{NAME_STEMS[category]}-{len(self.nodes)}.app"
        parent_path = self.nodes[parent]["path"] if parent is not None else ""
        path = f"{parent_path}/{name}" if parent_path else "/"
        size_log = min(max(self.rng.gauss(spec.size_log_mean, spec.size_log_sigma), 0.0), 1.0)
        age = min(max(self.rng.gauss(spec.age_mean, 0.012), 0.0), 1.0)
        access = min(max(self.rng.gauss(spec.access_mean, 0.010), 0.0), age)
        index = len(self.nodes)
        self.nodes.append({
            "category": category,
            "path": path,
            "name": name,
            "parent": parent,
            "size_log": size_log,
            "age": age,
            "access": access,
            "extension": extension,
            "is_dir": spec.is_dir,
            "hidden": spec.hidden or name.startswith("."),
            "executable": spec.executable,
            "safety": min(max(self.rng.gauss(spec.safety, 0.012), 0.0), 1.0),
            "anomaly": min(max(self.rng.gauss(spec.anomaly, 0.012), 0.0), 1.0),
        })
        self.children.append([])
        if parent is not None:
            self.children[parent].append(index)
            self.edges.extend(((parent, index), (index, parent)))
        return index

    def candidates(self, categories):
        return [index for index, node in enumerate(self.nodes) if node["category"] in categories and node["is_dir"]]

    def add_category(self, category):
        candidates = self.candidates(PARENT_CATEGORIES[category])
        if not candidates:
            directory_candidates = self.candidates(("directory", "root"))
            parent = self.rng.choice(directory_candidates)
        else:
            shallow = sorted(candidates, key=lambda index: self.depth(index))[:max(1, len(candidates) // 2)]
            parent = self.rng.choice(shallow)
        return self.add(category, parent)

    def depth(self, index):
        depth = 0
        parent = self.nodes[index]["parent"]
        while parent is not None:
            depth += 1
            parent = self.nodes[parent]["parent"]
        return depth

    def finish(self):
        max_depth = max(self.depth(index) for index in range(len(self.nodes)))
        features = []
        for index, node in enumerate(self.nodes):
            parent = node["parent"]
            features.append(make_node_features(
                node["size_log"], self.depth(index), max_depth, node["is_dir"], not node["is_dir"], False,
                node["hidden"], node["age"], node["access"], node["extension"], len(self.children[index]),
                node["executable"], parent is not None and self.nodes[parent]["is_dir"],
            ))
        for siblings in self.children:
            if len(siblings) > 1:
                ordered = siblings.copy()
                self.rng.shuffle(ordered)
                for left, right in zip(ordered, ordered[1:]):
                    self.edges.extend(((left, right), (right, left)))
        edge_index = torch.tensor(self.edges, dtype=torch.long).t().contiguous()
        return Data(
            x=torch.tensor(features, dtype=torch.float32),
            edge_index=edge_index,
            y=torch.tensor([LABEL_MAP[node["category"]] for node in self.nodes], dtype=torch.long),
            safety=torch.tensor([node["safety"] for node in self.nodes], dtype=torch.float32),
            anomaly=torch.tensor([node["anomaly"] for node in self.nodes], dtype=torch.float32),
            paths=[node["path"] for node in self.nodes],
            categories=[node["category"] for node in self.nodes],
            parents=torch.tensor([node["parent"] if node["parent"] is not None else -1 for node in self.nodes]),
        )


def generate_graph(rng, min_nodes=MIN_NODES, max_nodes=MAX_NODES):
    target_nodes = rng.randint(min_nodes, max_nodes)
    builder = TreeBuilder(rng)
    builder.add("root")
    directories = {}
    for name in ("Users", "Library", "Projects", "Downloads", "Documents", "Applications", "System"):
        index = builder.add("directory", 0)
        builder.nodes[index]["name"] = name
        builder.nodes[index]["path"] = f"/{name}"
        directories[name] = index
    builder.add("cache_dir", directories["Library"])
    builder.add("log_dir", directories["Library"])
    builder.add("build_output", directories["Projects"])
    builder.add("cargo_target", directories["Projects"])
    builder.add("library_dir", 0)
    required = [label for label, spec in CATEGORY_SPECS.items()
                if label not in {"root", "directory"} and rng.random() < spec.graph_probability]
    rng.shuffle(required)
    for category in required:
        if len(builder.nodes) < target_nodes:
            builder.add_category(category)
    categories = [label for label in LABEL_MAP if label != "root"]
    weights = [CATEGORY_SPECS[label].weight for label in categories]
    while len(builder.nodes) < target_nodes:
        builder.add_category(rng.choices(categories, weights=weights, k=1)[0])
    return builder.finish()


def generate_dataset(count, seed):
    rng = random.Random(seed)
    return [generate_graph(rng) for _ in range(count)]


def class_distribution(graphs):
    counts = Counter()
    graph_presence = Counter()
    for graph in graphs:
        labels = graph.y.tolist()
        counts.update(labels)
        graph_presence.update(set(labels))
    return {
        IDX_TO_LABEL[index]: {
            "nodes": counts[index],
            "node_fraction": counts[index] / sum(counts.values()),
            "graphs": graph_presence[index],
            "graph_fraction": graph_presence[index] / len(graphs),
        }
        for index in range(len(LABEL_MAP))
    }


def print_tree(graph, limit=80):
    for index, (path, category, parent) in enumerate(zip(graph.paths, graph.categories, graph.parents.tolist())):
        if index >= limit:
            print(f"... {graph.num_nodes - limit} more nodes")
            break
        depth = path.count("/") - (1 if path != "/" else 0)
        print(f"{'  ' * depth}{Path(path).name or '/'} [{category}] parent={parent}")


def validate_dataset(graphs):
    assert graphs
    assert all(graph.x.shape[1] == NUM_FEATURES for graph in graphs)
    assert all(MIN_NODES <= graph.num_nodes <= MAX_NODES for graph in graphs)
    assert all(torch.isfinite(graph.x).all() for graph in graphs)
    assert all(((graph.x >= 0.0) & (graph.x <= 1.0)).all() for graph in graphs)
    labels = {label for graph in graphs for label in graph.y.tolist()}
    assert labels <= set(range(len(LABEL_MAP)))
    if len(graphs) >= 100:
        assert labels == set(range(len(LABEL_MAP)))


def save_datasets(total=10_000, output_dir=DATA_DIR):
    train_count = int(total * 0.70)
    val_count = int(total * 0.15)
    test_count = total - train_count - val_count
    output_dir.mkdir(parents=True, exist_ok=True)
    splits = {
        "train": generate_dataset(train_count, 10_001),
        "val": generate_dataset(val_count, 20_003),
        "test": generate_dataset(test_count, 30_007),
    }
    for split, graphs in splits.items():
        validate_dataset(graphs)
        torch.save(graphs, output_dir / f"{split}.pt")
        print(f"{split}: graphs={len(graphs)} nodes={sum(graph.num_nodes for graph in graphs)}")
    distribution = class_distribution(splits["train"])
    with (output_dir / "class_distribution.json").open("w") as distribution_file:
        json.dump(distribution, distribution_file, indent=2)
    print_tree(splits["train"][0])
    return splits


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--graphs", type=int, default=10_000)
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR)
    parser.add_argument("--sample-only", action="store_true")
    args = parser.parse_args()
    if args.sample_only:
        graph = generate_graph(random.Random(42))
        validate_dataset([graph])
        print_tree(graph)
        return
    save_datasets(args.graphs, args.output_dir)


if __name__ == "__main__":
    main()
