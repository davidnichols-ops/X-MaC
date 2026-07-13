# Unified Memory Optimizer — Architecture Proposal

## A Proactive ML-Driven Middleware for Apple Silicon Memory Management

---

## 1. The Problem

Apple Silicon's unified memory architecture (UMA) is a breakthrough — CPU, GPU,
and Neural Engine share the same physical DRAM with zero-copy access. But the
**memory management strategy is fundamentally reactive**:

```
Free pages drop → Compress inactive pages → Compressor full → Swap to SSD → Jetsam kills processes
```

Each stage is a **reaction** to pressure that has already occurred. By the time
the compressor activates, the system is already under stress. By the time jetsam
fires, user-visible processes are being killed. The system has no mechanism to
**anticipate** pressure and act before it becomes critical.

Meanwhile, developers have no visibility into *why* memory is constrained. The
Activity Monitor shows aggregate numbers. There is no tool that says: "Process X
is growing at 50 MB/min and will trigger compression in 4 minutes" or "Your GPU
working set is at 80% of the limit and Metal is about to degrade."

**X-MaC already has the building blocks**: a 99.74%-accurate GNN for filesystem
analysis, CoreML on-device inference, memory stats collection, and a RAM boost
pipeline. The missing piece is **intelligence** — a layer that observes, predicts,
and acts proactively.

---

## 2. The Core Insight

### Model the Memory System as a Graph

The unified memory system is naturally a graph:

- **Nodes**: Processes, memory regions, hardware consumers (CPU clusters, GPU
  cores, ANE, media engines), swap files, compressor pool
- **Edges**: Ownership (process→region), sharing (IOSurface between processes),
  access (process→hardware), dependency (process→process via IPC/XPC)

This graph is **dynamic** — it evolves as processes allocate, free, share, and
terminate. The existing GAT (Graph Attention Network) architecture in X-MaC is
already designed for graph-structured data. We extend it from filesystem graphs
to **system memory graphs**.

### From Reactive to Proactive

Instead of waiting for pressure and reacting, the optimizer:

1. **Continuously observes** the memory graph (telemetry from all observable APIs)
2. **Predicts** pressure trajectories (which processes will grow, when
   compression will trigger, when jetsam will fire)
3. **Acts proactively** before the kernel's reactive systems engage
4. **Measures** the outcome and learns from feedback

This is the same shift that HALP (NSDI 2023) brought to YouTube CDN caches, and
that Voyager (ASPLOS 2021) brought to hardware prefetchers — but applied to
Apple Silicon's unified memory, which no existing research addresses.

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    X-MaC Memory Optimizer                         │
│                                                                  │
│  ┌───────────┐   ┌───────────────┐   ┌────────────┐              │
│  │  Telemetry │──▶│  Graph       │──▶│  GNN       │              │
│  │  Collector │   │  Builder     │   │  Inference │              │
│  │           │   │              │   │  Engine     │              │
│  │  (Rust)   │   │  (Rust)     │   │  (CoreML)  │              │
│  └───────────┘   └───────────────┘   └──────┬─────┘              │
│                                              │                    │
│                                              ▼                    │
│  ┌──────────────────────────────────┐  ┌────────────┐           │
│  │  Action Executor                  │◀─│  Policy   │           │
│  │  (Rust + privileged helper)       │  │  Engine   │           │
│  │                                   │  │  (Rust)  │           │
│  └──────────────────────────────────┘  └────────────┘           │
│                                              ▲                    │
│                                              │                    │
│  ┌──────────────────────────────────┐      │                    │
│  │  Feedback Loop                     │──────┘                    │
│  │  (measure before/after, update)   │                            │
│  └──────────────────────────────────┘                            │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Safety Guardrails                                       │   │
│  │  (protected processes, rate limits, dry-run mode)       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Layer 1: Telemetry Collector (Rust)

Collects memory data from all observable macOS APIs at configurable intervals
(default: 5 seconds, aggressive: 1 second).

**System-wide metrics** (via `host_statistics64` + `sysctl`):
- Page counts: free, active, inactive, speculative, wired, purgeable
- Compression: compressions, decompressions, compressor bytes used/pool size
- Swap: swapins, swapouts, swap usage
- Pressure level: `kern.memorystatus_vm_pressure_level`
- Page faults, COW faults, pageins, pageouts

**Per-process metrics** (via `task_info(TASK_VM_INFO)` + `proc_pidinfo`):
- Virtual size, resident size, physical footprint
- Compressed memory, purgeable volatile memory
- Internal, external, reusable, device memory
- Tagged ledgers: network, media, graphics, neural footprint
- Page faults, COW faults, pageins
- Thread count, priority, QoS class
- CPU time (user + system)

**GPU metrics** (via Metal + IOReport):
- `MTLDevice.recommendedMaxWorkingSetSize` (GPU memory ceiling)
- GPU wired memory (via `iogpu.wired_limit_mb` sysctl)
- GPU utilization (via IOReport channels)
- Metal resource count (if accessible)

**Hardware metrics** (via PMU/kperf, if root):
- Cache miss rates (L1, L2, SLC)
- Memory bandwidth utilization
- TLB miss rates

**Process tree** (via `proc_listallpids` + `proc_pidinfo`):
- Parent-child relationships
- Process groups
- Exec path, name, arguments

**Historical buffer**: Ring buffer keeping last N snapshots (default: 288
snapshots = 24 minutes at 5s intervals) for trend analysis.

### Layer 2: Graph Builder (Rust)

Transforms telemetry snapshots into a heterogeneous graph for GNN consumption.

**Node types**:

| Type | Features | Description |
|------|----------|-------------|
| Process | 24-dim | pid, RSS, virtual, footprint, compressed, growth_rate, qos, jetsam_band, thread_count, cpu_pct, age, is_foreground, is_system, ... |
| MemoryRegion | 12-dim | size, type (heap/stack/mmap/IOSurface), residency, compression_state, access_recency, protection, is_purgeable, is_shared, ... |
| HardwareConsumer | 8-dim | type (CPU/GPU/ANE/Media), utilization, bandwidth, working_set, power_state, ... |
| SwapFile | 6-dim | size, used, encrypted, creation_time, access_rate, ... |
| CompressorPool | 6-dim | bytes_used, pool_size, compression_ratio, input_rate, output_rate, ... |

**Edge types**:

| Type | Source → Target | Description |
|------|-----------------|-------------|
| OWNS | Process → MemoryRegion | Process allocated this region |
| SHARES | Process → MemoryRegion | Process has access (IOSurface, shared memory) |
| ACCESSES | Process → HardwareConsumer | Process uses GPU/ANE |
| PARENT_OF | Process → Process | Parent process relationship |
| DEPENDS_ON | Process → Process | IPC/XPC dependency |
| BACKS | MemoryRegion → SwapFile | Region is swapped to this file |
| COMPRESSED_IN | MemoryRegion → CompressorPool | Region is compressed here |

**Feature engineering**:

- **Growth rate**: `(RSS_t - RSS_{t-1}) / delta_t` (bytes/sec)
- **Acceleration**: Second derivative of RSS
- **Pressure trend**: Slope of free pages over last N snapshots
- **Compression efficiency**: `input_bytes / compressed_bytes` ratio
- **Working set ratio**: `resident / recommendedMaxWorkingSetSize` for GPU
- **Jetsam distance**: `phys_footprint / jetsam_limit` (how close to kill)

**Graph size**: Typically 50-300 nodes (processes + regions + hardware),
1,000-5,000 edges. Well within the existing GNN's capacity (trained on 50-200
node graphs, handles up to 600 via CoreML).

### Layer 3: GNN Inference Engine (CoreML)

Extends the existing GAT architecture for the memory domain.

**Model architecture** (MemoryGAT):

```
Input: Heterogeneous memory graph (50-300 nodes, 24-dim process features)
  │
  ├── Input projection: Linear(24 → 128)  [per node type]
  │
  ├── GAT Layer 1: GATConv(128 → 16 per head × 8 heads = 128)
  │   ├── LayerNorm + Residual + ELU
  │   └── Edge-type-aware attention (separate attention for each edge type)
  │
  ├── GAT Layer 2: GATConv(128 → 16 per head × 8 heads = 128)
  │   ├── LayerNorm + Residual + ELU
  │   └── Edge-type-aware attention
  │
  ├── GAT Layer 3: GATConv(128 → 16 per head × 8 heads = 128)
  │   ├── LayerNorm + Residual + ELU
  │   └── Edge-type-aware attention
  │
  └── Output heads:
      ├── Pressure head: Linear(128 → 3)  [normal/warn/critical in 60s]
      ├── Growth head: Linear(128 → 1)   [predicted RSS in 60s]
      ├── Action head: Linear(128 → 6)   [recommended action per process]
      └── Risk head: Linear(128 → 1)      [risk score for intervention]
```

**Predictions**:

1. **Pressure trajectory** (system-level): Probability of transitioning to
   warn/critical within 30s, 60s, 120s. This is the "early warning system."

2. **Per-process growth prediction**: Predicted RSS in 60s. Identifies processes
   that are about to cause pressure.

3. **Per-process action recommendation** (classification):
   - `NO_ACTION` — Process is healthy, leave alone
   - `PRESSURE_RELIEF` — Call `malloc_zone_pressure_relief` on this process
     (requires injection or cooperative API)
   - `SUGGEST_PURGE` — Process has purgeable memory, suggest purging
   - `DEPRIORITIZE` — Lower QoS / renice (background processes only)
   - `SUSPEND` — SIGSTOP (extreme cases, background only)
   - `TERMINATE` — Kill (last resort, with user confirmation)

4. **Risk score**: How risky is intervening with this process? System processes,
   foreground apps, and processes with active network connections score high
   risk.

**Training data**: Generated via:
- **Synthetic simulation**: Model memory allocation patterns, process lifecycles,
  pressure scenarios (extending the existing data_generator.py approach)
- **Real telemetry collection**: Run the telemetry collector on real workloads,
  label pressure events after the fact
- **Reinforcement feedback**: The feedback loop (Layer 6) provides reward
  signals: did the intervention help or hurt?

**CoreML distillation**: Same approach as existing — distill the 3-layer GAT
into a 3-layer MLP for per-node inference, enabling <1ms latency on-device.

### Layer 4: Policy Engine (Rust)

Translates GNN predictions into concrete actions, applying safety constraints.

**Policy inputs**:
- GNN predictions (pressure trajectory, per-process actions, risk scores)
- Current system state (pressure level, free pages, swap usage)
- User configuration (aggressiveness level, protected processes, dry-run mode)
- Historical outcomes (feedback loop data)

**Policy modes**:

| Mode | Trigger | Actions Allowed |
|------|---------|-----------------|
| `OBSERVE` | Default | Collect telemetry, predict, log. No actions. |
| `SUGGEST` | User enables | Show recommendations in GUI. User approves each action. |
| `GUARDED` | User enables | Auto-execute safe actions (purge, madvise). Never kill/suspend. |
| `PROACTIVE` | User enables + admin | Auto-execute all actions including suspend/kill (with guards). |

**Decision logic** (pseudocode):

```rust
fn decide(predictions: &Predictions, config: &Config) -> Vec<Action> {
    let mut actions = vec![];

    // System-level pressure prediction
    if predictions.pressure_60s > 0.7 && config.mode >= Mode::Guarded {
        // Proactive purge before kernel compressor engages
        actions.push(Action::PurgeDiskCache);
    }

    // Per-process actions
    for proc_pred in &predictions.processes {
        if proc_pred.risk_score > config.max_risk {
            continue;  // Skip risky interventions
        }

        if is_protected(&proc_pred.pid, &config.protected_list) {
            continue;
        }

        match proc_pred.recommended_action {
            ActionKind::PressureRelief if config.mode >= Mode::Guarded => {
                actions.push(Action::PressureRelief(proc_pred.pid));
            }
            ActionKind::Deprioritize if config.mode >= Mode::Guarded => {
                if proc_pred.is_background {
                    actions.push(Action::Renice(proc_pred.pid, +10));
                }
            }
            ActionKind::Suspend if config.mode >= Mode::Proactive => {
                if proc_pred.is_background && proc_pred.rss_mb > 500 {
                    actions.push(Action::Suspend(proc_pred.pid));
                }
            }
            ActionKind::Terminate if config.mode >= Mode::Proactive => {
                if proc_pred.risk_score < 0.3 && proc_pred.rss_mb > 1000 {
                    actions.push(Action::Terminate(proc_pred.pid));
                }
            }
            _ => {}
        }
    }

    // Rate limiting
    actions = rate_limit(actions, &config.max_actions_per_minute);

    // Dry-run filter
    if config.dry_run {
        for a in &mut actions {
            a.execute = false;
        }
    }

    actions
}
```

**Safety guardrails**:

- **Protected process list** (never touch): `kernel_task`, `launchd`,
  `WindowServer`, `Finder`, `Dock`, `SystemUIServer`, `loginwindow`,
  `coreaudiod`, `bluetoothd`, and any process matching user-defined patterns.
- **Rate limiting**: Max N actions per minute (default: 5), max M bytes freed
  per hour (default: 2 GB).
- **Cooldown**: After any intervention, wait 30s before next intervention on
  the same process.
- **Reversion**: If system state worsens after an action, automatically revert
  (e.g., SIGCONT after SIGSTOP, restore QoS).
- **Dry-run mode**: Log what *would* happen without executing. Default for new
  installations.
- **User confirmation**: In `SUGGEST` mode, every action requires GUI approval.

### Layer 5: Action Executor (Rust + Privileged Helper)

Executes approved actions via available macOS APIs.

**Available actions** (mapped to APIs from research):

| Action | API | Privilege | Reversible |
|--------|-----|-----------|------------|
| Purge disk cache | `purge` (via osascript or SMJobBless) | Admin | N/A |
| Process pressure relief | `malloc_zone->pressure_relief()` | Same process | Yes |
| Purgeable memory hint | `mach_vm_purgable_control(VOLATILE)` | Same process | Yes |
| madvise hint | `madvise(MADV_DONTNEED/MADV_FREE)` | Same process | Yes |
| Renice | `setpriority(PRIO_PROCESS, ...)` | Same user/root | Yes |
| QoS downgrade | `pthread_set_qos_class_self_np` | Same process | Yes |
| Suspend | `kill(SIGSTOP)` | Same user/root | Yes (SIGCONT) |
| Terminate | `kill(SIGTERM)` | Same user/root | No |
| Set footprint limit | `task_set_phys_footprint_limit()` | Same process | Yes |
| Memory behavior hint | `mach_vm_behavior_set(SEQUENTIAL)` | Same process | Yes |

**Privilege escalation paths**:

1. **AppleScript** (current approach): `do shell script "purge" with
   administrator privileges`. Simple but prompts each time.
2. **SMJobBless** (recommended for production): Install a privileged helper tool
   that runs as root via launchd. App communicates via XPC. One-time
   authentication, persistent helper. This is the Apple-recommended approach.
3. **Endpoint Security framework**: For monitoring process events system-wide.
   Requires Apple entitlement approval. Overkill for v1.

**Cross-process actions**: Actions like `pressure_relief` and `madvise` only
work on the calling process. For cross-process intervention, options are:
- Signal the target process (custom signal handler that calls pressure_relief)
- Use `task_for_pid` (requires root or same-user + entitlement)
- Cooperative API: X-MaC installs a library (via DYLD_INSERT_LIBRARIES) that
  listens for optimization signals. Too invasive for v1.

**v1 scope**: Focus on system-level actions (purge) and process-level actions
that don't require cross-process memory access (renice, suspend, terminate).
Cross-process memory optimization is a v2 goal.

### Layer 6: Feedback Loop (Rust)

Measures the outcome of every action and feeds it back into the model.

**Before/after measurement**:

```rust
struct ActionOutcome {
    action: Action,
    before: SystemSnapshot,
    after: SystemSnapshot,
    delta_free_pages: i64,
    delta_compressed: i64,
    delta_swap: i64,
    delta_pressure_level: i32,
    user_visible_impact: bool,  // Did any app become unresponsive?
    latency_ms: u64,            // Time from action to measurable effect
    success: bool,
}

fn measure_outcome(action: &Action, before: &SystemSnapshot) -> ActionOutcome {
    let after = SystemSnapshot::collect();
    let delta_free = after.free_pages as i64 - before.free_pages as i64;
    let delta_compressed = after.compressor_bytes as i64 - before.compressor_bytes as i64;
    let delta_swap = after.swap_used as i64 - before.swap_used as i64;

    ActionOutcome {
        action: action.clone(),
        before: before.clone(),
        after,
        delta_free_pages: delta_free,
        delta_compressed: delta_compressed,
        delta_swap: delta_swap,
        delta_pressure_level: after.pressure_level - before.pressure_level,
        user_visible_impact: check_app_responsiveness(),
        latency_ms: measure_latency(),
        success: delta_free > 0 && !user_visible_impact,
    }
}
```

**Reward signal** (for future RL training):
- Positive: Free pages increased, pressure level decreased, no user impact
- Negative: Free pages decreased, pressure increased, user-visible impact
- Neutral: No measurable change

**Feedback storage**: SQLite database storing all actions and outcomes. Used
for:
- Offline model retraining (periodic, not online)
- Policy tuning (adjust thresholds based on what worked)
- User reporting ("Your optimizer saved 2.3 GB this week, prevented 7 pressure
  events")

---

## 4. Data Flow

```
Time T=0:    Telemetry Collector gathers snapshot
             ↓
T=0.1s:      Graph Builder constructs memory graph (50-300 nodes)
             ↓
T=0.2s:      GNN Inference runs on CoreML (<1ms for 300 nodes)
             ↓
T=0.3s:      Policy Engine evaluates predictions + safety constraints
             ↓
T=0.4s:      If action needed: Action Executor runs (purge, renice, etc.)
             ↓
T=0.5s:      Feedback Loop measures before/after
             ↓
T=5.0s:      Next telemetry cycle begins
```

Total inference latency: <500ms (dominated by telemetry collection, not ML).
The GNN inference itself is <1ms, making real-time prediction feasible.

---

## 5. Integration with Existing X-MaC

### What Already Exists

| Component | Status | Reuse |
|-----------|--------|-------|
| GNN (GAT, 3-layer, 8-head) | Production (99.74% acc) | Extend architecture for memory domain |
| CoreML distillation pipeline | Working | Add memory model distillation |
| Memory stats collection (Rust) | Working | Extend with per-process + GPU metrics |
| RAM Boost (purge + kill) | Working | Becomes one action in the Action Executor |
| Activity logging | Working | Log all optimizer actions + outcomes |
| Swift GUI | Working | Add optimizer dashboard, predictions, action history |
| Graph extractor (Rust) | Working | Adapt for memory graph extraction |

### New Components Needed

| Component | Language | Effort | Description |
|-----------|----------|--------|-------------|
| Telemetry Collector | Rust | Medium | Extend `memory.rs` with per-process + GPU + PMU |
| Graph Builder | Rust | Medium | New module in `src/engines/optimize/` |
| Memory GNN model | Python | High | New training pipeline, data generator, model |
| Memory CoreML model | Python | Medium | Distillation + export (same pipeline as existing) |
| Policy Engine | Rust | Medium | New module, decision logic + safety |
| Action Executor | Rust | Medium | Wrap existing APIs, add SMJobBless helper |
| Feedback Loop | Rust | Low | Before/after measurement + SQLite storage |
| Optimizer GUI | Swift | Medium | Dashboard, predictions, action log, config |
| Privileged Helper | Swift/C | Medium | SMJobBless helper for root actions |

### Estimated Build Scope

**Phase 1 (MVP — Observe + Predict)**:
- Telemetry Collector (system + per-process)
- Graph Builder
- Memory GNN (train, distill, export to CoreML)
- GUI: Live memory graph visualization + pressure predictions
- No actions — pure observation and prediction

**Phase 2 (Suggest + Safe Actions)**:
- Policy Engine (SUGGEST + GUARDED modes)
- Action Executor (purge, madvise, renice)
- Feedback Loop
- GUI: Action recommendations, approval flow, outcome display

**Phase 3 (Proactive + Privileged)**:
- SMJobBless privileged helper
- Policy Engine (PROACTIVE mode)
- Action Executor (suspend, terminate with guards)
- GUI: Full optimizer dashboard, historical analytics, tuning

---

## 6. The Memory GNN — Detailed Design

### Why GNN for Memory?

The memory system is inherently graph-structured:
- Processes depend on each other (parent/child, IPC)
- Memory is shared between processes (IOSurface, mmap)
- Hardware consumers have contention relationships
- Pressure propagates through the graph (killing one process frees memory
  for others)

A GNN captures these **relational** patterns. A flat MLP (like the current
CoreML distillation) cannot reason about which processes are connected, which
are sharing memory, or which are causing cascading pressure.

### Training Data Generation

Extend `gnn/data_generator.py` with a **memory system simulator**:

```python
class MemorySystemSimulator:
    """Simulates a macOS system with processes, memory regions, and pressure."""

    def generate_graph(self) -> MemoryGraph:
        # Create process tree (launchd → daemons → user apps)
        # Assign memory regions to processes
        # Create IOSurface sharing relationships
        # Simulate allocation patterns (growth, stable, leaky)
        # Assign QoS classes and jetsam bands
        # Calculate system-wide pressure
        # Label: what's the optimal intervention?

    def simulate_pressure_scenario(self, scenario: str) -> MemoryGraph:
        scenarios = [
            "memory_leak",        # One process growing rapidly
            "cache_bloat",        # Many processes with large caches
            "gpu_pressure",      # GPU working set exceeds limit
            "swap_thrashing",    # Heavy swap activity
            "jetsam_cascade",    # Multiple processes near jetsam limit
            "healthy_system",    # Normal operation
            "startup_burst",     # Many processes starting simultaneously
            "idle_background",   # Many idle background processes
        ]
```

**Labels** (generated by simulation):
- Optimal action per process (determined by simulator: which intervention would
  have prevented the pressure event?)
- System pressure trajectory (simulated forward 60s)
- Process risk score (based on process type and state)

### Model Architecture Differences from Filesystem GNN

| Aspect | Filesystem GNN | Memory GNN |
|--------|---------------|------------|
| Node features | 16-dim (file properties) | 24-dim (process memory state) |
| Edge types | 2 (parent-child, sibling) | 7 (owns, shares, accesses, parent, depends, backs, compressed_in) |
| Node types | 1 (file/directory) | 5 (process, region, hardware, swap, compressor) |
| Output heads | 3 (class, safety, anomaly) | 4 (pressure, growth, action, risk) |
| Temporal | Static graph | Dynamic (time-series features) |
| Attention | Standard GAT | Edge-type-aware GAT |

**Edge-type-aware attention**: Each edge type gets its own attention weight
matrix. This is critical because the "OWNS" relationship has different semantics
than "SHARES" or "DEPENDS_ON". Implementation: separate `GATConv` per edge type,
sum the outputs.

**Temporal features**: Add rate-of-change features (growth rate, acceleration,
pressure trend) computed from the telemetry ring buffer. This gives the GNN
implicit temporal awareness without requiring a recurrent architecture.

### Distillation to CoreML

Same approach as existing filesystem GNN:
1. Train MemoryGAT (3-layer GAT, 8 heads) on synthetic + real data
2. Generate 10,000 distillation samples (node features → predicted outputs)
3. Train MemoryMLP (3-layer MLP, 128 hidden) as student
4. Export to `.mlpackage` via coremltools
5. Verify distillation MAE ≤ 0.05

**Inference path**: Rust collects telemetry → builds graph → extracts per-node
features → Swift calls CoreML → gets predictions → Rust executes policy.

---

## 7. Novelty and Significance

### What Makes This Novel

1. **First ML-based memory optimizer for Apple Silicon**: No existing tool or
   research applies ML to optimize unified memory on macOS. Existing tools
   (memory-optimiser, macmem, MemoryWatch) are threshold-based reactive
   monitors. Apple's own systems (compressor, jetsam) are reactive kernel
   mechanisms.

2. **Graph-based system modeling**: Modeling the entire memory system
   (processes + regions + hardware + swap + compressor) as a heterogeneous
   graph is novel. Existing ML memory work (HALP, LeCaR, PARROT) focuses on
   single-level cache eviction, not system-wide optimization.

3. **Proactive intervention**: The shift from reactive (wait for pressure →
   compress → swap → kill) to proactive (predict pressure → act before kernel
   engages) is the core contribution. This is the same paradigm shift that
   HALP brought to CDN caches, applied to a domain where it hasn't been done.

4. **Unified memory awareness**: The optimizer understands that CPU, GPU, and
   ANE share the same DRAM. It can detect when GPU working set is approaching
   the Metal limit, when ANE tensor allocations are competing with CPU heaps,
   and when IOSurface sharing is causing hidden memory pressure.

5. **Extends existing GNN infrastructure**: Rather than building from scratch,
   the architecture reuses X-MaC's production GNN (99.74% accuracy), CoreML
   pipeline, and memory collection — extending them to a new domain.

### What Makes This Feasible

1. **Observable APIs exist**: macOS provides rich memory observation
   (`host_statistics64`, `task_info`, `proc_pidinfo`, `sysctl`, Metal). No
   kernel modification needed for observation.

2. **Control APIs exist**: `purge`, `madvise`, `setpriority`, `task_set_phys_footprint_limit`,
   `mach_vm_purgable_control`, QoS classes, SIGSTOP/SIGCONT. Enough control
   for meaningful intervention without kernel access.

3. **GNN infrastructure exists**: The GAT architecture, training pipeline,
   CoreML distillation, and Swift integration are all working in X-MaC.

4. **Graph sizes are manageable**: 50-300 nodes is well within the GNN's
   proven capacity (trained on 50-200 node graphs, CoreML handles 600).

5. **Individual developer scope**: The MVP (observe + predict) requires
   extending existing Rust telemetry, building a new GNN training pipeline
   (reusing the existing one as template), and adding a GUI view. No kernel
   work, no Apple entitlements needed for v1.

### What Makes This Interesting

1. **Measurable impact**: Memory pressure events are quantifiable. "Prevented
   7 jetsam kills, saved 2.3 GB of swap, reduced compression by 40%" is
   concrete.

2. **Generalizable**: The architecture (observe → graph → predict → act →
   feedback) applies to any unified memory system. Apple Silicon is the
   testbed, but the approach generalizes to AMD APUs, Intel Xe, NVIDIA Grace
   Hopper.

3. **Research contribution**: A paper on "Graph Neural Networks for Proactive
   Memory Optimization on Apple Silicon" would be the first in this space.
   The combination of unified memory + GNN + proactive optimization is
   unexplored in the literature.

4. **Practical value**: Every Mac user experiences memory pressure. A tool
   that predicts and prevents it has immediate practical value, unlike much
   systems ML research that never ships.

---

## 8. Safety and Ethics

### What We Will NOT Do

- **No kernel modifications**: X-MaC is a user-space tool. No kext, no kernel
  patches, no SIP bypass.
- **No silent process killing**: Termination always requires user
  confirmation or explicit PROACTIVE mode opt-in.
- **No data collection**: All inference is on-device. No telemetry sent to
  any server. The feedback loop is local only.
- **No system integrity bypass**: No disabling SIP, no modifying system
  files, no injecting into system processes.
- **No performance regression**: The optimizer must never make the system
  slower. If an action doesn't improve things, it reverts.

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Killing critical process | Protected list + risk scoring + user confirmation |
| Causing more pressure | Feedback loop + automatic reversion + rate limiting |
| User-visible disruption | Foreground apps never touched without confirmation |
| Model inaccuracy | Confidence thresholds + dry-run default + fallback to heuristics |
| Privilege escalation attack | SMJobBless with code signing + minimal helper API |

---

## 9. Roadmap

### Phase 1: Observer (4-6 weeks)

**Goal**: Collect telemetry, build memory graph, predict pressure. No actions.

- [ ] Extend `src/util/memory.rs` with per-process metrics (`task_info`,
  `proc_pidinfo`)
- [ ] Add GPU memory metrics (Metal `recommendedMaxWorkingSetSize`, IOReport)
- [ ] Add telemetry ring buffer (configurable depth, default 288 snapshots)
- [ ] Create `src/engines/optimize/` module with graph builder
- [ ] Build memory system simulator in `gnn/data_generator_memory.py`
- [ ] Train MemoryGAT model (target: ≥90% pressure prediction accuracy)
- [ ] Distill to CoreML (target: ≤0.05 MAE, <5MB, <1ms inference)
- [ ] Add `MemoryGraphView` to Swift GUI (live graph, pressure gauge,
  predictions)
- [ ] Add `xmac optimize --observe` CLI mode

**Deliverable**: A tool that shows you the memory system as a graph and
predicts pressure events before they happen. No actions taken.

### Phase 2: Advisor (3-4 weeks)

**Goal**: Suggest actions, execute safe ones with user approval.

- [ ] Implement Policy Engine with SUGGEST mode
- [ ] Implement Action Executor for: purge, madvise, renice, QoS downgrade
- [ ] Implement Feedback Loop with SQLite storage
- [ ] Add action recommendation UI (SwiftGUI: cards with approve/reject)
- [ ] Add outcome display (before/after charts)
- [ ] Add `xmac optimize --suggest` CLI mode
- [ ] Add GUARDED mode (auto-execute safe actions)

**Deliverable**: A tool that recommends and executes safe memory
optimizations, showing you what it did and whether it helped.

### Phase 3: Proactive Optimizer (4-6 weeks)

**Goal**: Full proactive optimization with privileged helper.

- [ ] Implement SMJobBless privileged helper tool
- [ ] Add PROACTIVE mode (auto suspend/terminate with guards)
- [ ] Add historical analytics dashboard (weekly summary, trends)
- [ ] Add model retraining pipeline (offline, from feedback data)
- [ ] Add `xmac optimize --proactive` CLI mode
- [ ] Performance: sub-500ms full observe-predict-act cycle

**Deliverable**: A production proactive memory optimizer that runs
continuously, predicts pressure, and acts before the kernel's reactive
systems engage.

---

## 10. Technical References

### Apple Silicon Architecture
- "Meet the FaM1ly" — IEEE Micro 2022 (DOI: 10.1109/mm.2022.3169245)
- EXAM: Exploiting Exclusive SLC in Apple M-Series — USENIX/ACM 2024
- Orion: Characterizing Apple's Neural Engine — arXiv:2603.06728
- XNU source: apple-oss-distributions/xnu (vm_compressor.c, vm_pageout.c)

### ML for Memory Management
- HALP: ML-based cache eviction for YouTube CDN — NSDI 2023
- Voyager: Hierarchical neural data prefetcher — ASPLOS 2021
- Twilight: Reformulated neural prefetching — ISCA 2024
- Llama: ML-based memory allocation for C++ servers — ASPLOS 2020
- LeCaR: Regret minimization for cache replacement — HotStorage 2018
- PARROT: Imitation learning for cache eviction — ICML 2020
- LearnedCache: eBPF-integrated perceptron page cache — arXiv 2024

### GNN for Systems
- PROGRAML: Graph-based program representation — ICML 2021
- Transferable Graph Optimizers for ML compilers — NeurIPS 2020
- GNN for Radio Resource Management — JSAC 2021
- CoGNN: Algorithm-hardware co-design for GNN inference — 2021

### Apple Memory Management
- Mac Internals blog (macinternals.app): unified memory, IOSurface, GPU
- WWDC20: Explore new system architecture of Apple silicon
- WWDC22: Optimize your CoreML usage
- WWDC24: Analyze heap memory, Support real-time ML inference on CPU
- Locara: macOS Memory Management documentation

### Unified Memory Optimization
- Data Movement Is All You Need — MLSys 2021 (Outstanding Paper)
- Banshee: Bandwidth-efficient DRAM caching — MICRO 2017
- NVIDIA Unified Memory Programming Guide
- LLM in a Flash — Apple ML Research 2023
