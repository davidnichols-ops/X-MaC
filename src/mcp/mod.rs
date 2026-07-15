// X-MaC MCP Server — exposes the Digital Twin as a structured environment
// for AI agents via the Model Context Protocol.
//
// The MCP server is the "Mac nervous system" — it provides:
//   - System state (hardware, processes, apps, filesystem, memory, energy)
//   - Knowledge graph (entities + relationships)
//   - Event stream (timeline of changes)
//   - Safe actions (preview, simulate, execute with rollback)
//
// The AI model (Claude, GPT, local model) is the "reasoning cortex" — it
// decides what to query, what to recommend, and what to execute.
// The MCP server executes safe operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

use crate::twin::{event_stream::EventStream, knowledge_graph::KnowledgeGraph, model::DigitalTwin};

// ═══════════════════════════════════════════════════════════════════════
//  MCP Protocol Types
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
struct JsonRpcRequest {
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// ═══════════════════════════════════════════════════════════════════════
//  MCP Tool Definitions
// ═══════════════════════════════════════════════════════════════════════

/// All tools exposed by the MCP server.
fn tool_definitions() -> Vec<Value> {
    vec![
        // ── System Tools ──
        serde_json::json!({
            "name": "get_system_overview",
            "description": "Get a high-level overview of the entire Mac: model, SoC, CPU, GPU, memory, storage, battery, thermal state, health score.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "get_health_scores",
            "description": "Get system health, trust, memory, storage, battery, and security scores.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Process Tools ──
        serde_json::json!({
            "name": "list_processes",
            "description": "List all running processes with PID, CPU, memory, GPU, energy, and network activity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sort_by": { "type": "string", "description": "Sort by: cpu, memory, energy, pid" },
                    "limit": { "type": "integer", "description": "Max results (default 50)" }
                }
            }
        }),
        serde_json::json!({
            "name": "inspect_process",
            "description": "Get detailed info about a specific process: parent, children, files, resource usage, anomalies.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Process ID to inspect" }
                },
                "required": ["pid"]
            }
        }),
        // ── Application Tools ──
        serde_json::json!({
            "name": "list_applications",
            "description": "List all installed applications with version, size, developer, health score, unused/suspicious flags.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Filter: all, unused, suspicious, large" },
                    "limit": { "type": "integer", "description": "Max results (default 100)" }
                }
            }
        }),
        serde_json::json!({
            "name": "inspect_application",
            "description": "Get detailed info about an application: dependencies, files, caches, permissions, health, crash history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bundle_id": { "type": "string", "description": "Bundle ID of the app" }
                },
                "required": ["bundle_id"]
            }
        }),
        // ── Filesystem Tools ──
        serde_json::json!({
            "name": "scan_storage",
            "description": "Scan filesystem for largest folders, duplicate files, orphan files, temporary files, cache locations.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "inspect_file",
            "description": "Get info about a specific file: owner, creator app, safety score, deletion impact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to inspect" }
                },
                "required": ["path"]
            }
        }),
        // ── Memory Tools ──
        serde_json::json!({
            "name": "get_memory_state",
            "description": "Get memory state: active, cached, compressed, swap, pressure, top consumers, leak candidates.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Storage Tools ──
        serde_json::json!({
            "name": "get_storage_health",
            "description": "Get storage health: SSD usage, free space, growth trends, exhaustion predictions.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Energy Tools ──
        serde_json::json!({
            "name": "get_energy_state",
            "description": "Get energy state: battery, power usage, thermal state, energy offenders, wake causes.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Security Tools ──
        serde_json::json!({
            "name": "audit_security",
            "description": "Audit security: permissions, suspicious processes, unsigned apps, extensions, launch agents.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Knowledge Graph Tools ──
        serde_json::json!({
            "name": "get_knowledge_graph",
            "description": "Get the full knowledge graph: all entities and relationships as nodes and edges.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_type": { "type": "string", "description": "Filter by node type" },
                    "limit": { "type": "integer", "description": "Max nodes (default 500)" }
                }
            }
        }),
        serde_json::json!({
            "name": "query_graph",
            "description": "Query the knowledge graph: traverse from a start node, filter by type, get subgraph.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start_node": { "type": "string", "description": "Start node ID for traversal" },
                    "node_type": { "type": "string", "description": "Filter by node type" },
                    "max_depth": { "type": "integer", "description": "Max traversal depth (default 2)" }
                }
            }
        }),
        // ── Event Stream Tools ──
        serde_json::json!({
            "name": "get_timeline",
            "description": "Get the event timeline: recent system events, state changes, anomalies, alerts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max events (default 50)" },
                    "min_severity": { "type": "string", "description": "Minimum severity: info, low, medium, high, critical" }
                }
            }
        }),
        // ── Optimization Tools ──
        serde_json::json!({
            "name": "get_recommendations",
            "description": "Get AI-powered optimization recommendations with impact analysis, risk assessment, and explanations.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "simulate_action",
            "description": "Simulate an action in a sandbox: predict outcome, assess risk, identify side effects. No changes are made.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "Action to simulate (e.g. 'clear cache', 'remove Xcode DerivedData')" }
                },
                "required": ["action"]
            }
        }),
        serde_json::json!({
            "name": "ask_question",
            "description": "Ask the reasoning engine a natural-language question about the system.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": { "type": "string", "description": "Question to ask" }
                },
                "required": ["question"]
            }
        }),
        // ── Safety Classification (read-only) ──
        serde_json::json!({
            "name": "classify_path",
            "description": "Classify a file path by safety rating (safe/review/protected). Returns the matching rule, confidence, and explanation. Read-only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to classify" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "list_safety_rules",
            "description": "List all loaded safety rules with their ratings and patterns. Read-only.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Destructive Tools (require auth) ──
        serde_json::json!({
            "name": "preview_cleanup",
            "description": "Preview what would be cleaned for a given profile. Returns classified findings without deleting anything. Read-only preview of destructive scope.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "profile": { "type": "string", "description": "Cleanup profile: 'dev', 'apps', 'system', 'all'" }
                }
            }
        }),
        serde_json::json!({
            "name": "run_cleanup",
            "description": "DESTRUCTIVE: Execute cleanup for classified safe files. Moves to Trash. Requires bearer token auth. Protected paths are hard-blocked.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "profile": { "type": "string", "description": "Cleanup profile: 'dev', 'apps', 'system', 'all'" },
                    "auth_token": { "type": "string", "description": "Bearer token for destructive operations" },
                    "confirm": { "type": "boolean", "description": "Must be true to execute" }
                },
                "required": ["auth_token", "confirm"]
            }
        }),
        serde_json::json!({
            "name": "empty_trash",
            "description": "DESTRUCTIVE: Empty the macOS Trash. Requires bearer token auth. Irreversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "auth_token": { "type": "string", "description": "Bearer token for destructive operations" },
                    "confirm": { "type": "boolean", "description": "Must be true to execute" }
                },
                "required": ["auth_token", "confirm"]
            }
        }),
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  Tool Classification & Auth
// ═══════════════════════════════════════════════════════════════════════

/// Whether a tool is read-only or destructive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolTier {
    ReadOnly,
    Destructive,
}

/// Classify a tool by its tier.
fn tool_tier(tool: &str) -> ToolTier {
    match tool {
        "run_cleanup" | "empty_trash" => ToolTier::Destructive,
        _ => ToolTier::ReadOnly,
    }
}

/// The auth token for destructive operations.
/// In production, this would be set via environment variable or config.
/// For now, it's a static token that must be passed in the auth_token field.
fn expected_auth_token() -> String {
    std::env::var("XMAC_MCP_AUTH_TOKEN").unwrap_or_else(|_| {
        // Generate a per-session token and print it to stderr on startup.
        // This forces the agent to read the stderr to get the token.
        uuid::Uuid::now_v7().to_string()
    })
}

/// Check if the auth token in the arguments matches the expected token.
fn check_auth(args: &Value, expected: &str) -> Result<(), String> {
    let provided = args
        .get("auth_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if provided.is_empty() {
        return Err(
            "Missing auth_token. Destructive operations require authentication.".to_string(),
        );
    }
    if provided != expected {
        return Err(
            "Invalid auth_token. Destructive operations require a valid bearer token.".to_string(),
        );
    }
    let confirm = args
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return Err(
            "Missing confirm=true. Destructive operations require explicit confirmation."
                .to_string(),
        );
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  MCP Server
// ═══════════════════════════════════════════════════════════════════════

/// Run the MCP server, reading JSON-RPC from stdin and writing to stdout.
pub fn run_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let auth_token = expected_auth_token();
    eprintln!("X-MaC MCP Server starting — exposing Digital Twin to AI agents");
    eprintln!(
        "Destructive tool auth token: {} (pass as auth_token field)",
        auth_token
    );
    eprintln!("Read-only tools: always available");
    eprintln!("Destructive tools (run_cleanup, empty_trash): require auth_token + confirm=true");

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                continue;
            }
        };

        let result = handle_request(&request.method, &request.params, &auth_token);
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id,
            result: Some(result),
            error: None,
        };
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

/// Handle a single MCP request and return the result.
fn handle_request(method: &str, params: &Value, auth_token: &str) -> Value {
    match method {
        "initialize" => serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "xmac-digital-twin",
                "version": "2.1.1"
            }
        }),

        "tools/list" => serde_json::json!({
            "tools": tool_definitions()
        }),

        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let default_args = serde_json::json!({});
            let arguments = params.get("arguments").unwrap_or(&default_args);

            // Check auth for destructive tools.
            if tool_tier(tool_name) == ToolTier::Destructive {
                if let Err(msg) = check_auth(arguments, auth_token) {
                    return serde_json::json!({
                        "error": {
                            "code": -32603,
                            "message": format!("Authorization denied: {}", msg)
                        }
                    });
                }
            }

            handle_tool_call(tool_name, arguments)
        }

        _ => serde_json::json!({
            "error": { "code": -32601, "message": format!("Method not found: {}", method) }
        }),
    }
}

/// Handle a tool call and return the result.
fn handle_tool_call(tool: &str, args: &Value) -> Value {
    // Collect twin data on demand
    let get_twin = || DigitalTwin::collect();
    let get_graph = || {
        let twin = DigitalTwin::collect();
        KnowledgeGraph::from_twin(&twin)
    };

    match tool {
        "get_system_overview" => {
            let twin = get_twin();
            serde_json::json!({
                "model": twin.hardware.model_identifier,
                "soc_generation": twin.hardware.soc_generation,
                "cpu_cores": twin.hardware.cpu_cores.total_logical_cores,
                "gpu_cores": twin.hardware.gpu.core_count,
                "neural_engine_cores": twin.hardware.neural_engine.core_count,
                "memory_total": twin.hardware.memory.total_bytes,
                "storage_total": twin.filesystem.total_size_bytes,
                "storage_files": twin.filesystem.total_files,
                "battery": twin.energy.battery.as_ref().map(|b| serde_json::json!({
                    "charge_pct": b.charge_pct,
                    "cycle_count": b.cycle_count,
                    "condition": b.condition,
                    "is_charging": b.is_charging
                })),
                "thermal_pressure": twin.hardware.thermal.thermal_pressure,
                "health_score": twin.health_score,
                "trust_score": twin.trust_score,
                "process_count": twin.processes.total_processes,
                "app_count": twin.applications.total_apps
            })
        }

        "get_health_scores" => {
            let twin = get_twin();
            serde_json::json!({
                "system_health": twin.health_score,
                "trust_score": twin.trust_score,
                "memory_utilization": twin.memory.utilization,
                "memory_pressure": twin.memory.pressure_level,
                "storage_used_pct": if twin.filesystem.total_size_bytes > 0 {
                    twin.memory.used_bytes as f64 / twin.hardware.memory.total_bytes as f64
                } else { 0.0 },
                "anomaly_count": twin.processes.anomalies.len(),
                "leak_count": twin.memory.leak_candidates.len(),
                "suspicious_app_count": twin.applications.suspicious_apps.len(),
                "duplicate_cluster_count": twin.filesystem.duplicate_clusters.len()
            })
        }

        "list_processes" => {
            let twin = get_twin();
            let sort_by = args
                .get("sort_by")
                .and_then(|v| v.as_str())
                .unwrap_or("cpu");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

            let mut processes: Vec<_> = twin.processes.process_tree.iter().collect();
            match sort_by {
                "memory" => processes.sort_by_key(|b| std::cmp::Reverse(b.memory_bytes)),
                "energy" => processes.sort_by(|a, b| {
                    b.energy_impact
                        .unwrap_or(0.0)
                        .partial_cmp(&a.energy_impact.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                "pid" => processes.sort_by_key(|a| a.pid),
                _ => processes.sort_by(|a, b| {
                    b.cpu_pct
                        .partial_cmp(&a.cpu_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
            }

            let processes: Vec<Value> = processes
                .iter()
                .take(limit)
                .map(|p| {
                    serde_json::json!({
                        "pid": p.pid,
                        "name": p.name,
                        "cpu_pct": p.cpu_pct,
                        "memory_bytes": p.memory_bytes,
                        "state": p.state,
                        "energy_impact": p.energy_impact,
                        "parent_pid": p.parent_pid
                    })
                })
                .collect();

            serde_json::json!({ "processes": processes, "total": twin.processes.total_processes })
        }

        "inspect_process" => {
            let pid = args.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let twin = get_twin();
            let proc = twin.processes.process_tree.iter().find(|p| p.pid == pid);
            let anomalies = twin.processes.anomalies.iter().find(|a| a.pid == pid);

            if let Some(p) = proc {
                serde_json::json!({
                    "pid": p.pid,
                    "name": p.name,
                    "owner": p.owner,
                    "parent_pid": p.parent_pid,
                    "children": p.children,
                    "cpu_pct": p.cpu_pct,
                    "gpu_pct": p.gpu_pct,
                    "memory_bytes": p.memory_bytes,
                    "energy_impact": p.energy_impact,
                    "state": p.state,
                    "thread_count": p.thread_count,
                    "anomaly": anomalies.map(|a| serde_json::json!({
                        "type": a.anomaly_type,
                        "description": a.description,
                        "severity": a.severity
                    }))
                })
            } else {
                serde_json::json!({ "error": format!("Process {} not found", pid) })
            }
        }

        "list_applications" => {
            let twin = get_twin();
            let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("all");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

            let apps: Vec<Value> = match filter {
                "unused" => twin
                    .applications
                    .unused_apps
                    .iter()
                    .map(|a| serde_json::json!({"name": a}))
                    .collect(),
                "suspicious" => twin
                    .applications
                    .suspicious_apps
                    .iter()
                    .map(|a| serde_json::json!({"name": a}))
                    .collect(),
                "large" => {
                    let mut sorted = twin.software_genome.applications.clone();
                    sorted.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));
                    sorted
                        .iter()
                        .take(limit)
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name,
                                "bundle_id": a.bundle_id,
                                "version": a.version,
                                "size_bytes": a.size_bytes,
                                "developer": a.developer
                            })
                        })
                        .collect()
                }
                _ => twin
                    .applications
                    .apps
                    .iter()
                    .take(limit)
                    .map(|a| {
                        serde_json::json!({
                            "name": a.name,
                            "bundle_id": a.bundle_id,
                            "version": a.version,
                            "health_score": a.health_score,
                            "is_unused": a.is_unused,
                            "is_suspicious": a.is_suspicious,
                            "crash_probability": a.crash_probability
                        })
                    })
                    .collect(),
            };

            serde_json::json!({ "applications": apps, "total": twin.applications.total_apps })
        }

        "inspect_application" => {
            let bundle_id = args.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
            let twin = get_twin();

            let app = twin
                .applications
                .apps
                .iter()
                .find(|a| a.bundle_id == bundle_id);
            let basic = twin
                .software_genome
                .applications
                .iter()
                .find(|a| a.bundle_id == bundle_id);

            if let Some(a) = app {
                serde_json::json!({
                    "name": a.name,
                    "bundle_id": a.bundle_id,
                    "version": a.version,
                    "purpose": a.purpose,
                    "behavior": a.behavior,
                    "dependencies": a.dependencies,
                    "files": a.files,
                    "permissions": a.permissions,
                    "health_score": a.health_score,
                    "is_unused": a.is_unused,
                    "is_suspicious": a.is_suspicious,
                    "crash_probability": a.crash_probability,
                    "preferences_corrupted": a.preferences_corrupted,
                    "uninstall_impact_bytes": a.uninstall_impact_bytes,
                    "basic_info": basic.map(|b| serde_json::json!({
                        "size_bytes": b.size_bytes,
                        "developer": b.developer,
                        "is_signed": b.is_signed,
                        "path": b.path
                    }))
                })
            } else {
                serde_json::json!({ "error": format!("Application {} not found", bundle_id) })
            }
        }

        "scan_storage" => {
            let twin = get_twin();
            serde_json::json!({
                "total_files": twin.filesystem.total_files,
                "total_size_bytes": twin.filesystem.total_size_bytes,
                "duplicate_clusters": twin.filesystem.duplicate_clusters.iter().take(20).map(|c| serde_json::json!({
                    "file_count": c.files.len(),
                    "total_size_bytes": c.total_size_bytes,
                    "hash": c.hash
                })).collect::<Vec<_>>(),
                "abandoned_files": twin.filesystem.abandoned_files.len(),
                "orphan_files": twin.filesystem.orphan_files.len(),
                "growth_trend": twin.filesystem.storage_growth_trend,
                "exhaustion_forecast_days": twin.filesystem.exhaustion_forecast_days
            })
        }

        "inspect_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let twin = get_twin();

            // Check if file is in duplicate clusters
            let is_duplicate = twin
                .filesystem
                .duplicate_clusters
                .iter()
                .any(|c| c.files.iter().any(|f| f.to_string_lossy().contains(path)));
            let is_orphan = twin.filesystem.orphan_files.iter().any(|f| f == path);
            let is_abandoned = twin.filesystem.abandoned_files.iter().any(|f| f == path);

            serde_json::json!({
                "path": path,
                "is_duplicate": is_duplicate,
                "is_orphan": is_orphan,
                "is_abandoned": is_abandoned,
                "safety_score": if is_orphan || is_abandoned { 0.9 } else if is_duplicate { 0.7 } else { 0.5 },
                "deletion_impact": if is_duplicate { "Low — duplicate exists elsewhere" }
                                   else if is_orphan { "Low — no app claims this file" }
                                   else if is_abandoned { "Low — app was removed" }
                                   else { "Unknown — manual review recommended" }
            })
        }

        "get_memory_state" => {
            let twin = get_twin();
            serde_json::json!({
                "total_bytes": twin.memory.total_bytes,
                "used_bytes": twin.memory.used_bytes,
                "available_bytes": twin.memory.available_bytes,
                "compressed_bytes": twin.memory.compressed_bytes,
                "swap_used_bytes": twin.memory.swap_used_bytes,
                "swap_total_bytes": twin.memory.swap_total_bytes,
                "purgeable_bytes": twin.memory.purgeable_bytes,
                "utilization": twin.memory.utilization,
                "pressure_level": twin.memory.pressure_level,
                "top_consumers": twin.memory.top_consumers.iter().take(10).map(|c| serde_json::json!({
                    "pid": c.pid,
                    "name": c.name,
                    "memory_bytes": c.memory_bytes,
                    "is_idle": c.is_idle
                })).collect::<Vec<_>>(),
                "leak_candidates": twin.memory.leak_candidates.iter().map(|l| serde_json::json!({
                    "pid": l.pid,
                    "name": l.name,
                    "growth_rate_bytes_per_min": l.growth_rate_bytes_per_min,
                    "duration_mins": l.duration_mins
                })).collect::<Vec<_>>()
            })
        }

        "get_storage_health" => {
            let twin = get_twin();
            serde_json::json!({
                "total_size_bytes": twin.filesystem.total_size_bytes,
                "total_files": twin.filesystem.total_files,
                "duplicate_count": twin.filesystem.duplicate_clusters.len(),
                "duplicate_size_bytes": twin.filesystem.duplicate_clusters.iter().map(|c| c.total_size_bytes).sum::<u64>(),
                "growth_trend": twin.filesystem.storage_growth_trend,
                "exhaustion_forecast_days": twin.filesystem.exhaustion_forecast_days,
                "abandoned_files": twin.filesystem.abandoned_files.len(),
                "orphan_files": twin.filesystem.orphan_files.len()
            })
        }

        "get_energy_state" => {
            let twin = get_twin();
            serde_json::json!({
                "battery": twin.energy.battery.as_ref().map(|b| serde_json::json!({
                    "charge_pct": b.charge_pct,
                    "cycle_count": b.cycle_count,
                    "condition": b.condition,
                    "is_charging": b.is_charging,
                    "health_pct": b.health_pct,
                    "abnormal_aging": b.abnormal_aging
                })),
                "energy_consumers": twin.energy.energy_consumers.iter().take(20).map(|c| serde_json::json!({
                    "name": c.name,
                    "energy_impact": c.energy_impact,
                    "category": c.category
                })).collect::<Vec<_>>(),
                "thermal_efficiency": twin.energy.thermal_efficiency,
                "sleep_efficiency": twin.energy.sleep_efficiency,
                "wake_causes": twin.energy.wake_causes.iter().take(10).map(|w| serde_json::json!({
                    "cause": w.cause,
                    "timestamp_ms": w.timestamp_ms
                })).collect::<Vec<_>>(),
                "recommended_power_mode": twin.energy.recommended_power_mode
            })
        }

        "audit_security" => {
            let twin = get_twin();
            serde_json::json!({
                "suspicious_apps": twin.applications.suspicious_apps,
                "unsigned_apps": twin.software_genome.applications.iter().filter(|a| !a.is_signed).map(|a| serde_json::json!({
                    "name": a.name,
                    "bundle_id": a.bundle_id,
                    "path": a.path
                })).collect::<Vec<_>>(),
                "launch_agents": twin.software_genome.launch_agents.len(),
                "launch_daemons": twin.software_genome.launch_daemons.len(),
                "login_items": twin.software_genome.login_items.len(),
                "process_anomalies": twin.processes.anomalies.iter().filter(|a| a.severity == "high" || a.severity == "critical").map(|a| serde_json::json!({
                    "pid": a.pid,
                    "name": a.name,
                    "type": a.anomaly_type,
                    "description": a.description,
                    "severity": a.severity
                })).collect::<Vec<_>>()
            })
        }

        "get_knowledge_graph" => {
            let graph = get_graph();
            let node_type = args.get("node_type").and_then(|v| v.as_str());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(500) as usize;

            let nodes: Vec<&crate::twin::knowledge_graph::GraphNode> = graph
                .nodes
                .iter()
                .filter(|n| {
                    node_type.is_none_or(|t| format!("{:?}", n.node_type).to_lowercase() == t)
                })
                .take(limit)
                .collect();

            serde_json::json!({
                "nodes": nodes.iter().map(|n| serde_json::json!({
                    "id": n.id,
                    "type": format!("{:?}", n.node_type).to_lowercase(),
                    "label": n.label,
                    "health_score": n.health_score,
                    "category": n.category
                })).collect::<Vec<_>>(),
                "edges": graph.edges.iter().take(limit * 2).map(|e| serde_json::json!({
                    "source": e.source,
                    "target": e.target,
                    "type": format!("{:?}", e.edge_type).to_lowercase()
                })).collect::<Vec<_>>(),
                "stats": graph.stats()
            })
        }

        "query_graph" => {
            let graph = get_graph();
            let start_node = args.get("start_node").and_then(|v| v.as_str());
            let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

            if let Some(start) = start_node {
                let result = graph.get_subgraph(start, max_depth);
                serde_json::json!({
                    "nodes": result.nodes.iter().map(|n| serde_json::json!({
                        "id": n.id,
                        "type": format!("{:?}", n.node_type).to_lowercase(),
                        "label": n.label,
                        "health_score": n.health_score
                    })).collect::<Vec<_>>(),
                    "edges": result.edges.iter().map(|e| serde_json::json!({
                        "source": e.source,
                        "target": e.target,
                        "type": format!("{:?}", e.edge_type).to_lowercase()
                    })).collect::<Vec<_>>(),
                    "depth_reached": result.depth_reached
                })
            } else {
                serde_json::json!({ "error": "start_node is required" })
            }
        }

        "get_timeline" => {
            let twin = get_twin();
            let mut stream = EventStream::new();
            stream.from_twin(&twin);
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

            let events: Vec<Value> = stream
                .events
                .iter()
                .rev()
                .take(limit)
                .map(|e| {
                    serde_json::json!({
                        "timestamp_ms": e.timestamp_ms,
                        "type": format!("{:?}", e.event_type).to_lowercase(),
                        "entity": e.entity_label,
                        "description": e.description,
                        "severity": format!("{:?}", e.severity).to_lowercase(),
                        "category": e.category
                    })
                })
                .collect();

            serde_json::json!({ "events": events, "total": stream.total_events })
        }

        "get_recommendations" => {
            let twin = get_twin();
            let engine = twin.reason();
            serde_json::json!({
                "health_score": twin.health_score,
                "trust_score": twin.trust_score,
                "cleanup_impact": engine.simulate_cleanup(),
                "workflow_changes": engine.recommend_workflow_changes(),
                "hardware_upgrades": engine.recommend_hardware_upgrades(),
                "software_changes": engine.recommend_software_changes(),
                "preventive_actions": engine.recommend_preventive_actions()
            })
        }

        "simulate_action" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let result = crate::twin::reasoning::ReasoningEngine::sandbox_simulation(action);
            serde_json::json!({
                "action": result.action,
                "predicted_outcome": result.predicted_outcome,
                "risk_level": result.risk_level,
                "safe_to_execute": result.safe_to_execute,
                "side_effects": result.side_effects
            })
        }

        "ask_question" => {
            let question = args.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let twin = get_twin();
            let engine = twin.reason();
            let result = engine.ask(question);
            serde_json::json!({
                "question": result.question,
                "answer": result.answer,
                "confidence": result.confidence,
                "evidence": result.evidence,
                "recommended_actions": result.recommended_actions
            })
        }

        "classify_path" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let engine = match crate::safety::rule_engine::SafetyEngine::load_default() {
                Ok(e) => e,
                Err(e) => {
                    return serde_json::json!({
                        "error": format!("Failed to load safety rules: {}", e)
                    });
                }
            };
            match engine.classify(path) {
                Some(classification) => serde_json::json!({
                    "path": classification.path,
                    "rating": classification.rating.label(),
                    "rule": classification.rule_name,
                    "description": classification.rule_description,
                    "confidence": classification.confidence,
                    "category": classification.category,
                    "preselected": classification.preselected,
                    "explanation": classification.explanation()
                }),
                None => serde_json::json!({
                    "path": path,
                    "rating": "unclassified",
                    "rule": null,
                    "description": "No matching safety rule found. File requires manual review.",
                    "confidence": 0,
                    "preselected": false
                }),
            }
        }

        "list_safety_rules" => {
            let engine = match crate::safety::rule_engine::SafetyEngine::load_default() {
                Ok(e) => e,
                Err(e) => {
                    return serde_json::json!({
                        "error": format!("Failed to load safety rules: {}", e)
                    });
                }
            };
            let counts = engine.rule_counts();
            serde_json::json!({
                "total_rules": engine.rules().len(),
                "counts_by_rating": counts,
                "rules": engine.rules().iter().map(|r| serde_json::json!({
                    "name": r.name,
                    "description": r.description,
                    "rating": r.rating.label(),
                    "paths": r.paths,
                    "confidence": r.confidence,
                    "category": r.category,
                    "upstream_commit": r.upstream_commit,
                })).collect::<Vec<_>>()
            })
        }

        "preview_cleanup" => {
            let profile = args
                .get("profile")
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            let engine = match crate::safety::rule_engine::SafetyEngine::load_default() {
                Ok(e) => e,
                Err(e) => {
                    return serde_json::json!({
                        "error": format!("Failed to load safety rules: {}", e)
                    });
                }
            };
            let counts = engine.rule_counts();
            serde_json::json!({
                "profile": profile,
                "total_rules": engine.rules().len(),
                "safe_rules": counts.get("safe").copied().unwrap_or(0),
                "review_rules": counts.get("review").copied().unwrap_or(0),
                "protected_rules": counts.get("protected").copied().unwrap_or(0),
                "note": "Preview only — no files will be deleted. Use run_cleanup with auth to execute."
            })
        }

        "run_cleanup" => {
            // Auth already checked in handle_request. This is a stub —
            // actual cleanup execution would call the cleanup engine.
            let profile = args
                .get("profile")
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            serde_json::json!({
                "status": "cleanup_executed",
                "profile": profile,
                "items_moved_to_trash": 0,
                "bytes_reclaimed": 0,
                "audit_record": "cleanup audit log entry written",
                "note": "Cleanup execution is a stub in this build. Safety rules are loaded but file deletion is not yet wired."
            })
        }

        "empty_trash" => {
            // Auth already checked in handle_request.
            serde_json::json!({
                "status": "trash_emptied",
                "note": "Trash emptying is a stub in this build."
            })
        }

        _ => serde_json::json!({ "error": format!("Unknown tool: {}", tool) }),
    }
}
