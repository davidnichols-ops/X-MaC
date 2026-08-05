# X-MaC — Operations Manifest

630 technical operations mapped to X-MaC engine/module locations.
Each op has: ID, name, target engine/module, status (existing/extend/new), and implementation notes.

Legend: `[E]` = existing, `[X]` = extend, `[N]` = new

---

## Part A: Cleaner/Optimizer Operations (1–300)

### 1. Filesystem Discovery & Storage Intelligence (1–30)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 1 | Traverse APFS volumes | `engines/disk/engine.rs` | `[E]` | Already traverses via walkdir |
| 2 | Enumerate mounted disks | `engines/disk/engine.rs` | `[E]` | df -k parsing in system_awareness |
| 3 | Read filesystem metadata | `engines/disk/engine.rs` | `[E]` | stat/metadata calls |
| 4 | Build directory trees | `engines/disk/engine.rs` | `[E]` | walkdir traversal |
| 5 | Calculate folder sizes | `engines/disk/engine.rs` | `[E]` | Recursive size calc |
| 6 | Calculate file sizes | `core/types.rs` | `[E]` | size_bytes field |
| 7 | Track file creation dates | `engines/clean/scanner.rs` | `[X]` | Add creation date to findings metadata |
| 8 | Track modification dates | `engines/clean/scanner.rs` | `[E]` | Already used for age-based filtering |
| 9 | Track access dates | `engines/clean/scanner.rs` | `[X]` | Add atime tracking |
| 10 | Identify hidden files | `engines/clean/scanner.rs` | `[E]` | include_hidden config |
| 11 | Identify system files | `engines/clean/rules.rs` | `[E]` | Category classification |
| 12 | Identify user files | `engines/clean/rules.rs` | `[E]` | Category classification |
| 13 | Classify file extensions | `engines/clean/rules.rs` | `[E]` | Extension-based rules |
| 14 | Classify MIME types | `engines/clean/rules.rs` | `[X]` | Add MIME type detection |
| 15 | Build storage heat maps | `engines/disk/engine.rs` | `[N]` | New: density visualization data |
| 16 | Generate treemap visualization | `gui/XMacApp/DiskView.swift` | `[N]` | Roadmap v2.2: Space Lens |
| 17 | Detect storage hotspots | `engines/disk/engine.rs` | `[N]` | New: anomalous density detection |
| 18 | Rank largest directories | `engines/disk/engine.rs` | `[E]` | Already sorts by size |
| 19 | Rank oldest files | `engines/clean/scanner.rs` | `[X]` | Add age ranking output |
| 20 | Rank unused files | `engines/clean/scanner.rs` | `[X]` | Add access-time ranking |
| 21 | Detect removable media | `engines/disk/engine.rs` | `[X]` | Add disk type detection |
| 22 | Scan external drives | `engines/disk/engine.rs` | `[X]` | Extend volume enumeration |
| 23 | Scan network mounts | `engines/disk/engine.rs` | `[X]` | Filter network filesystems |
| 24 | Scan cloud-synced folders | `engines/clean/scanner.rs` | `[N]` | Roadmap v2.3: cloud cleanup |
| 25 | Detect iCloud Drive data | `engines/clean/scanner.rs` | `[N]` | ~/Library/Mobile Documents |
| 26 | Detect Dropbox data | `engines/clean/scanner.rs` | `[N]` | ~/Dropbox |
| 27 | Detect Google Drive data | `engines/clean/scanner.rs` | `[N]` | ~/Google Drive |
| 28 | Detect OneDrive data | `engines/clean/scanner.rs` | `[N]` | ~/OneDrive |
| 29 | Cache filesystem scan results | `engines/disk/engine.rs` | `[X]` | Add result caching |
| 30 | Incremental rescanning | `engines/disk/engine.rs` | `[X]` | Add hash-based incremental scan |

### 2. Cache Analysis Engine (31–60)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 31 | Locate user caches | `engines/clean/scanner.rs` | `[E]` | ~/Library/Caches |
| 32 | Locate system caches | `engines/clean/scanner.rs` | `[E]` | /Library/Caches |
| 33 | Browser cache detection | `engines/clean/rules.rs` | `[E]` | BrowserCache category |
| 34 | Safari cache cleanup | `engines/clean/rules.rs` | `[E]` | Safari paths |
| 35 | Chrome cache cleanup | `engines/clean/rules.rs` | `[E]` | Chrome paths |
| 36 | Firefox cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Firefox paths |
| 37 | Chromium cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Chromium variants |
| 38 | Electron app cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Electron detection |
| 39 | Slack cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Slack cache paths |
| 40 | Discord cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Discord cache paths |
| 41 | Steam cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Steam paths |
| 42 | Adobe cache cleanup | `engines/clean/rules.rs` | `[X]` | Add Adobe paths |
| 43 | Xcode cache cleanup | `engines/clean/rules.rs` | `[E]` | XcodeArtifact category |
| 44 | Remove shader caches | `engines/clean/rules.rs` | `[X]` | Add shader cache paths |
| 45 | Remove thumbnail caches | `engines/clean/rules.rs` | `[X]` | Add .DS_Store, thumbnails |
| 46 | Remove preview caches | `engines/clean/rules.rs` | `[X]` | Add Quick Look previews |
| 47 | Remove QuickLook cache | `engines/maintain/engine.rs` | `[E]` | Already in maintain |
| 48 | Remove font cache | `engines/maintain/engine.rs` | `[X]` | Add font cache clearing |
| 49 | Remove application temporary data | `engines/clean/scanner.rs` | `[E]` | TempFile category |
| 50 | Detect cache age | `engines/clean/scanner.rs` | `[X]` | Add age metadata to cache findings |
| 51 | Detect cache regeneration ability | `engines/clean/rules.rs` | `[N]` | New: classify regenerability |
| 52 | Estimate reclaimable space | `core/types.rs` | `[E]` | total_reclaimable_bytes |
| 53 | Protect active caches | `engines/clean/rules.rs` | `[X]` | Add active cache protection |
| 54 | Ignore protected folders | `engines/clean/scanner.rs` | `[E]` | exclude_patterns |
| 55 | Detect corrupted caches | `engines/clean/scanner.rs` | `[N]` | New: cache integrity check |
| 56 | Rebuild cache indexes | `engines/maintain/engine.rs` | `[N]` | New: cache index rebuild |
| 57 | Remove stale cache databases | `engines/clean/rules.rs` | `[X]` | Add stale DB detection |
| 58 | Purge temporary directories | `engines/clean/scanner.rs` | `[E]` | /tmp, var/folders |
| 59 | Analyze cache ownership | `engines/clean/scanner.rs` | `[X]` | Add owner metadata |
| 60 | Validate deletion safety | `cleanup/policy.rs` | `[E]` | CleanupPolicy validation |

### 3. Log Management (61–80)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 61 | Find application logs | `engines/clean/scanner.rs` | `[E]` | ~/Library/Logs |
| 62 | Find system logs | `engines/clean/scanner.rs` | `[E]` | /Library/Logs |
| 63 | Remove crash reports | `engines/clean/rules.rs` | `[X]` | Add crash report paths |
| 64 | Remove diagnostic logs | `engines/clean/rules.rs` | `[X]` | Add diagnostic log paths |
| 65 | Remove installer logs | `engines/clean/rules.rs` | `[X]` | Add installer log paths |
| 66 | Remove update logs | `engines/clean/rules.rs` | `[X]` | Add update log paths |
| 67 | Remove browser logs | `engines/clean/rules.rs` | `[X]` | Add browser log paths |
| 68 | Detect oversized logs | `engines/clean/scanner.rs` | `[X]` | Add size threshold for logs |
| 69 | Compress old logs | `cleanup/executor.rs` | `[N]` | New: log compression before deletion |
| 70 | Determine log age | `engines/clean/scanner.rs` | `[E]` | mtime-based |
| 71 | Determine log importance | `engines/clean/rules.rs` | `[N]` | New: log importance classifier |
| 72 | Preserve security logs | `engines/clean/rules.rs` | `[X]` | Add security log whitelist |
| 73 | Remove debug files | `engines/clean/rules.rs` | `[X]` | Add debug file patterns |
| 74 | Clean developer logs | `engines/clean/rules.rs` | `[X]` | Add dev log paths |
| 75 | Clean simulator logs | `engines/clean/rules.rs` | `[X]` | Add iOS Simulator logs |
| 76 | Clean Xcode build logs | `engines/clean/rules.rs` | `[E]` | XcodeArtifact category |
| 77 | Clean package manager logs | `engines/clean/rules.rs` | `[X]` | Add Homebrew, npm, pip logs |
| 78 | Clean Homebrew logs | `engines/clean/rules.rs` | `[X]` | ~/Library/Logs/Homebrew |
| 79 | Clean Python cache logs | `engines/clean/rules.rs` | `[X]` | __pycache__, .pytest_cache |
| 80 | Clean Docker logs | `engines/clean/rules.rs` | `[X]` | Docker log paths |

### 4. Duplicate Detection Engine (81–110)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 81 | Scan file candidates | `engines/duplicate/engine.rs` | `[N]` | New engine |
| 82 | Compare filenames | `engines/duplicate/engine.rs` | `[N]` | Name-based pre-filter |
| 83 | Compare file sizes | `engines/duplicate/engine.rs` | `[N]` | Size-based grouping |
| 84 | Generate hashes | `engines/duplicate/engine.rs` | `[N]` | BLAKE3 (already a dep) |
| 85 | Generate partial hashes | `engines/duplicate/engine.rs` | `[N]` | First/last N bytes |
| 86 | Generate full hashes | `engines/duplicate/engine.rs` | `[N]` | Full file BLAKE3 |
| 87 | Compare SHA hashes | `engines/duplicate/engine.rs` | `[N]` | Hash comparison |
| 88 | Compare metadata | `engines/duplicate/engine.rs` | `[N]` | Metadata comparison |
| 89 | Detect identical files | `engines/duplicate/engine.rs` | `[N]` | Hash match |
| 90 | Detect renamed duplicates | `engines/duplicate/engine.rs` | `[N]` | Same hash, different name |
| 91 | Detect copied folders | `engines/duplicate/engine.rs` | `[N]` | Folder-level hash comparison |
| 92 | Group duplicate clusters | `engines/duplicate/engine.rs` | `[N]` | Union-find clustering |
| 93 | Detect duplicate photos | `engines/duplicate/engine.rs` | `[N]` | Image-specific detection |
| 94 | Detect similar photos | `engines/duplicate/engine.rs` | `[N]` | Perceptual hashing |
| 95 | Generate image fingerprints | `engines/duplicate/engine.rs` | `[N]` | pHash implementation |
| 96 | Compare perceptual hashes | `engines/duplicate/engine.rs` | `[N]` | Hamming distance |
| 97 | Detect screenshots | `engines/duplicate/engine.rs` | `[N]` | Screenshot pattern detection |
| 98 | Detect blurry images | `engines/duplicate/engine.rs` | `[N]` | Laplacian variance |
| 99 | Rank duplicate confidence | `engines/duplicate/engine.rs` | `[N]` | Confidence scoring |
| 100 | Select safest deletion candidate | `engines/duplicate/engine.rs` | `[N]` | Keep newest/largest/original |
| 101 | Preserve newest file | `engines/duplicate/engine.rs` | `[N]` | mtime-based selection |
| 102 | Preserve highest-resolution image | `engines/duplicate/engine.rs` | `[N]` | Dimension-based selection |
| 103 | Preserve original path | `engines/duplicate/engine.rs` | `[N]` | Path priority |
| 104 | Create deletion queue | `engines/duplicate/engine.rs` | `[N]` | Queue for cleanup pipeline |
| 105 | Move to Trash | `cleanup/transaction.rs` | `[E]` | Trash-first cleanup |
| 106 | Secure delete | `cleanup/transaction.rs` | `[N]` | New: secure overwrite option |
| 107 | Undo deletion | `cleanup/undo.rs` | `[E]` | Undo support exists |
| 108 | Generate duplicate reports | `engines/duplicate/engine.rs` | `[N]` | Report generation |
| 109 | Ignore duplicate whitelist | `engines/duplicate/engine.rs` | `[N]` | User-configured whitelist |
| 110 | Maintain scan database | `engines/duplicate/engine.rs` | `[N]` | Hash database persistence |

### 5. Application Intelligence (111–145)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 111 | Enumerate installed applications | `engines/envmap/apps.rs` | `[E]` | /Applications scanning |
| 112 | Read application bundles | `engines/envmap/apps.rs` | `[E]` | .app/Contents parsing |
| 113 | Identify app version | `engines/envmap/apps.rs` | `[E]` | Info.plist version |
| 114 | Identify developer | `engines/envmap/apps.rs` | `[X]` | Add developer extraction |
| 115 | Identify bundle identifier | `engines/envmap/apps.rs` | `[E]` | CFBundleIdentifier |
| 116 | Identify app size | `engines/envmap/apps.rs` | `[X]` | Add size calculation |
| 117 | Identify last launch time | `engines/envmap/apps.rs` | `[N]` | New: launch history tracking |
| 118 | Detect unused applications | `engines/envmap/apps.rs` | `[N]` | New: usage-based detection |
| 119 | Detect abandoned applications | `engines/envmap/apps.rs` | `[N]` | New: age + no launch |
| 120 | Find application leftovers | `engines/envmap/discovery.rs` | `[X]` | Extend with leftover detection |
| 121 | Find preferences | `engines/envmap/discovery.rs` | `[E]` | ~/Library/Preferences |
| 122 | Find containers | `engines/envmap/discovery.rs` | `[E]` | ~/Library/Containers |
| 123 | Find support files | `engines/envmap/discovery.rs` | `[E]` | ~/Library/Application Support |
| 124 | Find saved states | `engines/envmap/discovery.rs` | `[X]` | Add Saved Application State |
| 125 | Find plugins | `engines/envmap/discovery.rs` | `[X]` | Add plugin directories |
| 126 | Find extensions | `engines/envmap/discovery.rs` | `[X]` | Add extension directories |
| 127 | Find login helpers | `engines/envmap/discovery.rs` | `[X]` | Add LoginItems/Helpers |
| 128 | Find app caches | `engines/envmap/discovery.rs` | `[E]` | ~/Library/Caches/<bundle> |
| 129 | Remove application completely | `cleanup/transaction.rs` | `[X]` | Extend with app uninstall |
| 130 | Remove orphan files | `engines/envmap/discovery.rs` | `[N]` | New: orphan detection |
| 131 | Detect broken uninstallations | `engines/envmap/discovery.rs` | `[N]` | New: broken uninstall detection |
| 132 | Remove old installers | `engines/clean/rules.rs` | `[X]` | Add DMG/PKG detection |
| 133 | Detect DMG files | `engines/clean/rules.rs` | `[X]` | Add .dmg scanning |
| 134 | Detect PKG installers | `engines/clean/rules.rs` | `[X]` | Add .pkg scanning |
| 135 | Detect old versions | `engines/envmap/apps.rs` | `[N]` | New: version comparison |
| 136 | Compare app versions | `engines/envmap/apps.rs` | `[N]` | New: semver comparison |
| 137 | Track application history | `engines/envmap/apps.rs` | `[N]` | New: install/remove tracking |
| 138 | Recommend removals | `intelligence/advisor.rs` | `[X]` | Extend advisor with app recs |
| 139 | Protect system apps | `engines/envmap/apps.rs` | `[E]` | Apple app protection |
| 140 | Protect Apple apps | `engines/envmap/apps.rs` | `[E]` | Apple bundle ID protection |
| 141 | Detect unsigned apps | `engines/envmap/apps.rs` | `[N]` | New: code signature check |
| 142 | Detect suspicious apps | `engines/envmap/apps.rs` | `[N]` | New: heuristic suspicious detection |
| 143 | Detect duplicate applications | `engines/envmap/apps.rs` | `[N]` | New: duplicate app detection |
| 144 | Generate app inventory | `engines/envmap/apps.rs` | `[X]` | Add inventory export |
| 145 | Export application report | `cli/output.rs` | `[X]` | Add app report format |

### 6. Startup & Background Process Management (146–175)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 146 | Scan Login Items | `engines/startup/engine.rs` | `[N]` | New engine |
| 147 | Scan LaunchAgents (user) | `engines/startup/engine.rs` | `[N]` | ~/Library/LaunchAgents |
| 148 | Scan LaunchAgents (system) | `engines/startup/engine.rs` | `[N]` | /Library/LaunchAgents |
| 149 | Scan LaunchDaemons | `engines/startup/engine.rs` | `[N]` | /Library/LaunchDaemons |
| 150 | Parse plist files | `engines/startup/engine.rs` | `[N]` | plist crate (already a dep) |
| 151 | Identify startup commands | `engines/startup/engine.rs` | `[N]` | ProgramArguments extraction |
| 152 | Identify background helpers | `engines/startup/engine.rs` | `[N]` | Helper app detection |
| 153 | Identify update agents | `engines/startup/engine.rs` | `[N]` | Update agent classification |
| 154 | Identify telemetry processes | `engines/startup/engine.rs` | `[N]` | Telemetry classification |
| 155 | Identify unnecessary services | `engines/startup/engine.rs` | `[N]` | Heuristic necessity scoring |
| 156 | Disable startup items | `engines/startup/engine.rs` | `[N]` | launchctl bootout |
| 157 | Enable startup items | `engines/startup/engine.rs` | `[N]` | launchctl bootstrap |
| 158 | Remove startup entries | `engines/startup/engine.rs` | `[N]` | Plist removal (trash-first) |
| 159 | Detect delayed launch apps | `engines/startup/engine.rs` | `[N]` | StartInterval detection |
| 160 | Measure boot impact | `engines/startup/engine.rs` | `[N]` | Boot time measurement |
| 161 | Estimate CPU impact | `engines/startup/engine.rs` | `[N]` | CPU impact estimation |
| 162 | Estimate memory impact | `engines/startup/engine.rs` | `[N]` | Memory impact estimation |
| 163 | Monitor background processes | `engines/optimize/telemetry.rs` | `[E]` | Process monitoring exists |
| 164 | Monitor CPU usage | `engines/optimize/telemetry.rs` | `[E]` | CPU tracking exists |
| 165 | Monitor RAM usage | `engines/optimize/telemetry.rs` | `[E]` | Memory tracking exists |
| 166 | Monitor network usage | `engines/optimize/telemetry.rs` | `[X]` | Add network monitoring |
| 167 | Monitor disk activity | `engines/optimize/telemetry.rs` | `[X]` | Add disk I/O monitoring |
| 168 | Detect runaway processes | `intelligence/advisor.rs` | `[E]` | Runaway process detection |
| 169 | Recommend optimization | `intelligence/advisor.rs` | `[E]` | Recommendation engine |
| 170 | Kill frozen applications | `engines/startup/engine.rs` | `[N]` | New: frozen app termination |
| 171 | Restart services | `engines/startup/engine.rs` | `[N]` | New: service restart |
| 172 | Monitor daemon health | `engines/startup/engine.rs` | `[N]` | New: daemon health check |
| 173 | Detect zombie processes | `engines/startup/engine.rs` | `[N]` | New: zombie detection |
| 174 | Track process history | `engines/optimize/telemetry.rs` | `[X]` | Add historical tracking |
| 175 | Generate performance reports | `cli/output.rs` | `[X]` | Add performance report format |

### 7. macOS Database Maintenance (176–205)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 176 | Rebuild Spotlight index | `engines/maintain/engine.rs` | `[E]` | mdutil integration |
| 177 | Repair Spotlight metadata | `engines/maintain/engine.rs` | `[X]` | Add metadata repair |
| 178 | Rebuild LaunchServices database | `engines/maintain/engine.rs` | `[E]` | lsregister |
| 179 | Refresh Finder metadata | `engines/maintain/engine.rs` | `[X]` | Add Finder refresh |
| 180 | Reset Finder preferences | `engines/maintain/engine.rs` | `[N]` | New: Finder prefs reset |
| 181 | Reset Dock database | `engines/maintain/engine.rs` | `[N]` | New: Dock reset |
| 182 | Clear QuickLook cache | `engines/maintain/engine.rs` | `[E]` | qlmanage -m cache remove |
| 183 | Reset font database | `engines/maintain/engine.rs` | `[N]` | New: font database reset |
| 184 | Clear icon cache | `engines/maintain/engine.rs` | `[N]` | New: icon cache clear |
| 185 | Reset application associations | `engines/maintain/engine.rs` | `[X]` | Add UTI reset |
| 186 | Repair permissions metadata | `engines/maintain/engine.rs` | `[N]` | New: permissions repair |
| 187 | Validate APFS volume | `engines/maintain/engine.rs` | `[N]` | New: APFS validation |
| 188 | Run filesystem checks | `engines/maintain/engine.rs` | `[N]` | New: fsck integration |
| 189 | Trigger maintenance scripts | `engines/maintain/engine.rs` | `[E]` | periodic scripts |
| 190 | Rotate logs | `engines/maintain/engine.rs` | `[N]` | New: log rotation |
| 191 | Clean periodic tasks | `engines/maintain/engine.rs` | `[E]` | Periodic cleanup |
| 192 | Refresh system databases | `engines/maintain/engine.rs` | `[X]` | Add system DB refresh |
| 193 | Clear stale preferences | `engines/maintain/engine.rs` | `[N]` | New: stale prefs clear |
| 194 | Reset application state | `engines/maintain/engine.rs` | `[N]` | New: app state reset |
| 195 | Clear recent items | `engines/maintain/engine.rs` | `[N]` | New: recent items clear |
| 196 | Remove broken aliases | `engines/depth/symlink.rs` | `[X]` | Extend with alias detection |
| 197 | Repair file associations | `engines/maintain/engine.rs` | `[N]` | New: UTI repair |
| 198 | Repair services database | `engines/maintain/engine.rs` | `[N]` | New: services DB repair |
| 199 | Refresh system cache | `engines/maintain/engine.rs` | `[X]` | Add system cache refresh |
| 200 | Trigger OS housekeeping | `engines/maintain/engine.rs` | `[X]` | Add OS housekeeping |
| 201 | Verify system integrity | `engines/maintain/engine.rs` | `[N]` | New: SIP verification |
| 202 | Analyze disk health | `engines/maintain/engine.rs` | `[N]` | New: disk health analysis |
| 203 | Read SMART data | `engines/maintain/engine.rs` | `[N]` | New: SMART data reading |
| 204 | Monitor disk errors | `engines/maintain/engine.rs` | `[N]` | New: disk error monitoring |
| 205 | Detect filesystem anomalies | `engines/maintain/engine.rs` | `[N]` | New: anomaly detection |

### 8. Privacy & Security Operations (206–235)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 206 | Clear browser history | `engines/privacy/engine.rs` | `[N]` | New engine |
| 207 | Clear cookies | `engines/privacy/engine.rs` | `[N]` | New engine |
| 208 | Clear downloads history | `engines/privacy/engine.rs` | `[N]` | New engine |
| 209 | Clear autofill data | `engines/privacy/engine.rs` | `[N]` | New engine |
| 210 | Clear recent documents | `engines/privacy/engine.rs` | `[N]` | New engine |
| 211 | Clear application history | `engines/privacy/engine.rs` | `[N]` | New engine |
| 212 | Clear chat traces | `engines/privacy/engine.rs` | `[N]` | New engine |
| 213 | Remove tracking data | `engines/privacy/engine.rs` | `[N]` | New engine |
| 214 | Remove temporary credentials | `engines/privacy/engine.rs` | `[N]` | New engine |
| 215 | Scan malware signatures | `engines/privacy/engine.rs` | `[N]` | New engine |
| 216 | Scan suspicious applications | `engines/privacy/engine.rs` | `[N]` | New engine |
| 217 | Check permissions | `engines/privacy/engine.rs` | `[N]` | New engine |
| 218 | Check accessibility permissions | `engines/privacy/engine.rs` | `[N]` | New engine |
| 219 | Check full disk access permissions | `engines/privacy/engine.rs` | `[N]` | New engine |
| 220 | Detect vulnerable apps | `engines/privacy/engine.rs` | `[N]` | New engine |
| 221 | Detect outdated software | `engines/privacy/engine.rs` | `[N]` | New engine |
| 222 | Identify risky extensions | `engines/privacy/engine.rs` | `[N]` | New engine |
| 223 | Remove adware | `engines/privacy/engine.rs` | `[N]` | New engine |
| 224 | Remove browser hijackers | `engines/privacy/engine.rs` | `[N]` | New engine |
| 225 | Secure file deletion | `cleanup/transaction.rs` | `[N]` | New: secure delete |
| 226 | Overwrite deleted files | `cleanup/transaction.rs` | `[N]` | New: overwrite option |
| 227 | Remove metadata | `engines/privacy/engine.rs` | `[N]` | New engine |
| 228 | Strip EXIF information | `engines/privacy/engine.rs` | `[N]` | New engine |
| 229 | Detect suspicious processes | `engines/privacy/engine.rs` | `[N]` | New engine |
| 230 | Monitor security events | `engines/privacy/engine.rs` | `[N]` | New engine |
| 231 | Generate security report | `engines/privacy/engine.rs` | `[N]` | New engine |
| 232 | Quarantine files | `engines/privacy/engine.rs` | `[N]` | New engine |
| 233 | Restore quarantined files | `engines/privacy/engine.rs` | `[N]` | New engine |
| 234 | Maintain threat database | `engines/privacy/engine.rs` | `[N]` | New engine |
| 235 | Update detection rules | `engines/privacy/engine.rs` | `[N]` | New engine |

### 9. Memory & Performance Optimization (236–260)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 236 | Read RAM statistics | `intelligence/system_awareness.rs` | `[E]` | MemoryDimension |
| 237 | Read memory pressure | `intelligence/system_awareness.rs` | `[E]` | pressure_level |
| 238 | Read swap usage | `intelligence/system_awareness.rs` | `[E]` | swap_used_bytes |
| 239 | Read compressed memory | `intelligence/system_awareness.rs` | `[E]` | compressed_bytes |
| 240 | Read purgeable memory | `engines/optimize/telemetry.rs` | `[X]` | Add purgeable memory |
| 241 | Identify memory-heavy apps | `engines/optimize/telemetry.rs` | `[E]` | Top processes |
| 242 | Identify idle applications | `engines/optimize/telemetry.rs` | `[X]` | Add idle detection |
| 243 | Suggest quitting apps | `intelligence/advisor.rs` | `[E]` | App quit recommendations |
| 244 | Flush caches | `engines/maintain/engine.rs` | `[E]` | RAM purge |
| 245 | Trigger memory compression | `engines/optimize/engine.rs` | `[X]` | Add compression trigger |
| 246 | Monitor CPU load | `intelligence/system_awareness.rs` | `[E]` | CpuDimension |
| 247 | Monitor GPU load | `intelligence/system_awareness.rs` | `[X]` | Add GPU monitoring |
| 248 | Monitor thermal state | `intelligence/system_awareness.rs` | `[E]` | ThermalDimension |
| 249 | Monitor battery health | `intelligence/system_awareness.rs` | `[E]` | BatteryDimension |
| 250 | Monitor energy impact | `intelligence/system_awareness.rs` | `[X]` | Add energy impact |
| 251 | Monitor fan activity | `intelligence/system_awareness.rs` | `[X]` | Add fan monitoring |
| 252 | Detect overheating | `intelligence/advisor.rs` | `[E]` | Thermal recommendations |
| 253 | Detect battery drain | `intelligence/advisor.rs` | `[E]` | Battery recommendations |
| 254 | Optimize background tasks | `intelligence/daemon.rs` | `[E]` | Daemon automation |
| 255 | Reduce login overhead | `engines/startup/engine.rs` | `[N]` | Login item optimization |
| 256 | Analyze slow boots | `engines/startup/engine.rs` | `[N]` | Boot time analysis |
| 257 | Profile application startup | `engines/optimize/engine.rs` | `[N]` | New: app startup profiling |
| 258 | Track performance trends | `intelligence/daemon.rs` | `[X]` | Add trend tracking |
| 259 | Generate optimization score | `intelligence/system_awareness.rs` | `[E]` | health_score |
| 260 | Recommend actions | `intelligence/advisor.rs` | `[E]` | Recommendation engine |

### 10. AI / Next-Gen Optimization Layer (261–300)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 261 | Build complete system graph | `engines/graph/engine.rs` | `[E]` | GraphBuilder exists |
| 262 | Map file ownership | `twin/fs_graph.rs` | `[N]` | Twin filesystem graph |
| 263 | Understand application behavior | `twin/app_agent.rs` | `[N]` | Twin app agent |
| 264 | Predict safe deletion | `engines/graph/engine.rs` | `[E]` | GNN safety scoring |
| 265 | Predict cache regeneration | `engines/clean/rules.rs` | `[N]` | New: regen prediction |
| 266 | Learn user habits | `intelligence/advisor.rs` | `[E]` | AdaptiveState learning |
| 267 | Detect abnormal growth | `twin/fs_graph.rs` | `[N]` | Twin: growth detection |
| 268 | Predict storage exhaustion | `twin/fs_graph.rs` | `[N]` | Twin: storage forecasting |
| 269 | Detect runaway caches | `twin/fs_graph.rs` | `[N]` | Twin: cache growth |
| 270 | Detect memory leaks | `twin/memory.rs` | `[N]` | Twin: leak detection |
| 271 | Detect inefficient apps | `twin/app_agent.rs` | `[N]` | Twin: efficiency analysis |
| 272 | Recommend workflow changes | `twin/reasoning.rs` | `[N]` | Twin: workflow recs |
| 273 | Rank optimization impact | `intelligence/advisor.rs` | `[X]` | Extend with impact ranking |
| 274 | Create system health score | `intelligence/system_awareness.rs` | `[E]` | health_score 0-100 |
| 275 | Maintain historical baseline | `intelligence/daemon.rs` | `[X]` | Add baseline tracking |
| 276 | Detect performance regression | `twin/reasoning.rs` | `[N]` | Twin: regression detection |
| 277 | Compare before/after states | `intelligence/zen.rs` | `[E]` | Zen before/after |
| 278 | Build machine profile | `twin/hardware.rs` | `[N]` | Twin: hardware profile |
| 279 | Understand Apple Silicon architecture | `twin/hardware.rs` | `[N]` | Twin: SoC detection |
| 280 | Optimize unified memory usage | `twin/memory.rs` | `[N]` | Twin: unified memory |
| 281 | Analyze Neural Engine workloads | `twin/hardware.rs` | `[N]` | Twin: ANE analysis |
| 282 | Analyze GPU workloads | `twin/hardware.rs` | `[N]` | Twin: GPU analysis |
| 283 | Analyze Metal resources | `twin/hardware.rs` | `[N]` | Twin: Metal analysis |
| 284 | Optimize developer environments | `twin/reasoning.rs` | `[N]` | Twin: dev env optimization |
| 285 | Optimize Docker environments | `engines/map/containers.rs` | `[X]` | Extend container map |
| 286 | Optimize ML workloads | `twin/reasoning.rs` | `[N]` | Twin: ML optimization |
| 287 | Optimize Xcode workflows | `engines/clean/rules.rs` | `[X]` | Extend Xcode cleanup |
| 288 | Optimize gaming workloads | `twin/reasoning.rs` | `[N]` | Twin: gaming optimization |
| 289 | Detect unused ML models | `twin/software_genome.rs` | `[N]` | Twin: ML model inventory |
| 290 | Detect duplicate datasets | `engines/duplicate/engine.rs` | `[N]` | Duplicate engine: datasets |
| 291 | Manage large AI caches | `engines/clean/rules.rs` | `[X]` | Add AI cache paths |
| 292 | Manage package caches | `engines/clean/rules.rs` | `[E]` | PackageManagerCache category |
| 293 | Manage Python environments | `engines/map/python.rs` | `[E]` | Python env mapping |
| 294 | Manage npm caches | `engines/map/nodejs.rs` | `[E]` | Node env mapping |
| 295 | Manage Rust build artifacts | `engines/clean/rules.rs` | `[X]` | Add target/ scanning |
| 296 | Manage container storage | `engines/map/containers.rs` | `[E]` | Container mapping |
| 297 | Automate maintenance schedules | `intelligence/daemon.rs` | `[E]` | Daemon scheduling |
| 298 | Explain every recommendation | `intelligence/advisor.rs` | `[E]` | NL explanations |
| 299 | Create reversible optimization plans | `cleanup/transaction.rs` | `[E]` | Undo support |
| 300 | Act as a macOS "systems operator" | `twin/reasoning.rs` | `[N]` | Twin: autonomous operator |

---

## Part B: Digital Twin Operations (1–330)

### 11. Hardware Reality Model (1–40)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 1 | Identify exact Mac model | `twin/hardware.rs` | `[N]` | sysctl hw.model |
| 2 | Identify SoC generation | `twin/hardware.rs` | `[N]` | sysctl hw.optional |
| 3 | Detect CPU cores | `intelligence/system_awareness.rs` | `[E]` | sysctl hw.logicalcpu |
| 4 | Detect performance cores | `twin/hardware.rs` | `[N]` | sysctl hw.perflevel0.physicalcpu |
| 5 | Detect efficiency cores | `twin/hardware.rs` | `[N]` | sysctl hw.perflevel1.physicalcpu |
| 6 | Detect GPU cores | `twin/hardware.rs` | `[N]` | sysctl hw.gpuuuid / Metal |
| 7 | Detect Neural Engine cores | `twin/hardware.rs` | `[N]` | ANE detection |
| 8 | Detect unified memory size | `twin/hardware.rs` | `[N]` | sysctl hw.memsize |
| 9 | Detect memory bandwidth | `twin/hardware.rs` | `[N]` | sysctl hw.memfrequency |
| 10 | Detect SSD capacity | `twin/hardware.rs` | `[N]` | diskutil info |
| 11 | Detect SSD health | `twin/hardware.rs` | `[N]` | SMART data |
| 12 | Detect battery model | `twin/hardware.rs` | `[N]` | system_profiler SPPowerDataType |
| 13 | Detect battery cycles | `intelligence/system_awareness.rs` | `[E]` | cycle_count |
| 14 | Detect battery chemistry | `twin/hardware.rs` | `[N]` | system_profiler |
| 15 | Detect thermal sensors | `twin/hardware.rs` | `[N]` | SMC access |
| 16 | Detect fan controllers | `twin/hardware.rs` | `[N]` | SMC access |
| 17 | Detect display characteristics | `twin/hardware.rs` | `[N]` | system_profiler SPDisplaysDataType |
| 18 | Detect external displays | `twin/hardware.rs` | `[N]` | system_profiler |
| 19 | Detect connected peripherals | `twin/hardware.rs` | `[N]` | system_profiler SPUSBDataType |
| 20 | Detect USB topology | `twin/hardware.rs` | `[N]` | ioreg |
| 21 | Detect Thunderbolt topology | `twin/hardware.rs` | `[N]` | system_profiler SPThunderboltDataType |
| 22 | Detect Bluetooth devices | `twin/hardware.rs` | `[N]` | system_profiler SPBluetoothDataType |
| 23 | Detect WiFi hardware | `twin/hardware.rs` | `[N]` | system_profiler SPNetworkDataType |
| 24 | Detect network interfaces | `twin/hardware.rs` | `[N]` | ifconfig |
| 25 | Detect audio devices | `twin/hardware.rs` | `[N]` | system_profiler SPAudioDataType |
| 26 | Detect camera devices | `twin/hardware.rs` | `[N]` | system_profiler SPCameraDataType |
| 27 | Detect microphone devices | `twin/hardware.rs` | `[N]` | system_profiler SPMicrophoneDataType |
| 28 | Detect GPU utilization | `twin/hardware.rs` | `[N]` | Metal/GPU counters |
| 29 | Detect CPU utilization | `intelligence/system_awareness.rs` | `[E]` | CpuDimension |
| 30 | Detect memory pressure | `intelligence/system_awareness.rs` | `[E]` | MemoryDimension |
| 31 | Detect swap activity | `intelligence/system_awareness.rs` | `[E]` | swap_used_bytes |
| 32 | Detect SSD throughput | `twin/hardware.rs` | `[N]` | disk performance metrics |
| 33 | Detect power state | `twin/hardware.rs` | `[N]` | pmset -g |
| 34 | Detect sleep states | `twin/hardware.rs` | `[N]` | pmset -g assertions |
| 35 | Detect wake events | `twin/hardware.rs` | `[N]` | log show --predicate |
| 36 | Detect thermal throttling | `twin/hardware.rs` | `[N]` | pmset -g therm |
| 37 | Detect performance limits | `twin/hardware.rs` | `[N]` | sysctl hw.cpufrequency_max |
| 38 | Build hardware capability profile | `twin/hardware.rs` | `[N]` | Aggregated profile |
| 39 | Create machine fingerprint | `twin/hardware.rs` | `[N]` | Hardware hash |
| 40 | Maintain hardware history | `twin/hardware.rs` | `[N]` | Historical tracking |

### 12. Complete Software Genome (41–80)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 41 | Inventory every installed application | `twin/software_genome.rs` | `[N]` | Twin: app inventory |
| 42 | Inventory every executable | `twin/software_genome.rs` | `[N]` | Twin: executable scan |
| 43 | Inventory frameworks | `twin/software_genome.rs` | `[N]` | Twin: framework scan |
| 44 | Inventory dynamic libraries | `twin/software_genome.rs` | `[N]` | Twin: dylib scan |
| 45 | Inventory kernel extensions | `twin/software_genome.rs` | `[N]` | Twin: kext scan |
| 46 | Inventory system extensions | `twin/software_genome.rs` | `[N]` | Twin: sysextd scan |
| 47 | Inventory launch agents | `engines/startup/engine.rs` | `[N]` | Startup engine |
| 48 | Inventory launch daemons | `engines/startup/engine.rs` | `[N]` | Startup engine |
| 49 | Inventory login items | `engines/startup/engine.rs` | `[N]` | Startup engine |
| 50 | Inventory background services | `engines/startup/engine.rs` | `[N]` | Startup engine |
| 51 | Inventory plugins | `twin/software_genome.rs` | `[N]` | Twin: plugin scan |
| 52 | Inventory browser extensions | `twin/software_genome.rs` | `[N]` | Twin: extension scan |
| 53 | Inventory fonts | `twin/software_genome.rs` | `[N]` | Twin: font scan |
| 54 | Inventory developer tools | `twin/software_genome.rs` | `[N]` | Twin: dev tools scan |
| 55 | Inventory SDKs | `twin/software_genome.rs` | `[N]` | Twin: SDK scan |
| 56 | Inventory package managers | `engines/map/engine.rs` | `[E]` | Map engine |
| 57 | Inventory Python environments | `engines/map/python.rs` | `[E]` | Python env mapping |
| 58 | Inventory Node environments | `engines/map/nodejs.rs` | `[E]` | Node env mapping |
| 59 | Inventory Rust toolchains | `twin/software_genome.rs` | `[N]` | Twin: rustup scan |
| 60 | Inventory Docker images | `engines/map/containers.rs` | `[E]` | Container mapping |
| 61 | Inventory containers | `engines/map/containers.rs` | `[E]` | Container mapping |
| 62 | Inventory virtual machines | `twin/software_genome.rs` | `[N]` | Twin: VM scan |
| 63 | Inventory AI models | `twin/software_genome.rs` | `[N]` | Twin: ML model scan |
| 64 | Inventory datasets | `twin/software_genome.rs` | `[N]` | Twin: dataset scan |
| 65 | Inventory games | `twin/software_genome.rs` | `[N]` | Twin: game scan |
| 66 | Inventory game launchers | `twin/software_genome.rs` | `[N]` | Twin: launcher scan |
| 67 | Inventory cloud applications | `twin/software_genome.rs` | `[N]` | Twin: cloud app scan |
| 68 | Inventory synchronization clients | `twin/software_genome.rs` | `[N]` | Twin: sync client scan |
| 69 | Track software versions | `twin/software_genome.rs` | `[N]` | Twin: version tracking |
| 70 | Track update history | `twin/software_genome.rs` | `[N]` | Twin: update history |
| 71 | Track install dates | `twin/software_genome.rs` | `[N]` | Twin: install tracking |
| 72 | Track removal dates | `twin/software_genome.rs` | `[N]` | Twin: removal tracking |
| 73 | Track application relationships | `twin/software_genome.rs` | `[N]` | Twin: relationship graph |
| 74 | Build software dependency graph | `twin/software_genome.rs` | `[N]` | Twin: dependency graph |
| 75 | Identify obsolete software | `twin/software_genome.rs` | `[N]` | Twin: obsolescence detection |
| 76 | Identify conflicting software | `engines/conflict/engine.rs` | `[E]` | Conflict engine |
| 77 | Identify redundant software | `twin/software_genome.rs` | `[N]` | Twin: redundancy detection |
| 78 | Identify risky software | `engines/privacy/engine.rs` | `[N]` | Privacy engine |
| 79 | Identify unused software | `twin/software_genome.rs` | `[N]` | Twin: usage detection |
| 80 | Maintain software map | `twin/software_genome.rs` | `[N]` | Twin: software map |

### 13. Filesystem Intelligence Graph (81–120)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 81 | Index every file | `twin/fs_graph.rs` | `[N]` | Twin: file index |
| 82 | Map file ownership | `twin/fs_graph.rs` | `[N]` | Twin: ownership graph |
| 83 | Map file creators | `twin/fs_graph.rs` | `[N]` | Twin: creator mapping |
| 84 | Map application relationships | `twin/fs_graph.rs` | `[N]` | Twin: app-file graph |
| 85 | Map file dependencies | `twin/fs_graph.rs` | `[N]` | Twin: dependency graph |
| 86 | Map cache relationships | `twin/fs_graph.rs` | `[N]` | Twin: cache graph |
| 87 | Map configuration relationships | `twin/fs_graph.rs` | `[N]` | Twin: config graph |
| 88 | Map preference relationships | `twin/fs_graph.rs` | `[N]` | Twin: preference graph |
| 89 | Map temporary files | `twin/fs_graph.rs` | `[N]` | Twin: temp file graph |
| 90 | Map generated files | `twin/fs_graph.rs` | `[N]` | Twin: generated file detection |
| 91 | Map source files | `twin/fs_graph.rs` | `[N]` | Twin: source file mapping |
| 92 | Map compiled artifacts | `twin/fs_graph.rs` | `[N]` | Twin: artifact mapping |
| 93 | Map documents | `twin/fs_graph.rs` | `[N]` | Twin: document mapping |
| 94 | Map media | `twin/fs_graph.rs` | `[N]` | Twin: media mapping |
| 95 | Map archives | `twin/fs_graph.rs` | `[N]` | Twin: archive mapping |
| 96 | Map backups | `twin/fs_graph.rs` | `[N]` | Twin: backup mapping |
| 97 | Map cloud files | `twin/fs_graph.rs` | `[N]` | Twin: cloud file mapping |
| 98 | Map duplicate content | `engines/duplicate/engine.rs` | `[N]` | Duplicate engine |
| 99 | Map near-duplicate content | `engines/duplicate/engine.rs` | `[N]` | Duplicate engine |
| 100 | Predict file importance | `twin/fs_graph.rs` | `[N]` | Twin: importance prediction |
| 101 | Predict deletion safety | `engines/graph/engine.rs` | `[E]` | GNN safety scoring |
| 102 | Detect abandoned files | `twin/fs_graph.rs` | `[N]` | Twin: abandoned detection |
| 103 | Detect orphan files | `engines/envmap/discovery.rs` | `[N]` | Orphan detection |
| 104 | Detect forgotten downloads | `twin/fs_graph.rs` | `[N]` | Twin: download detection |
| 105 | Detect storage leaks | `twin/fs_graph.rs` | `[N]` | Twin: leak detection |
| 106 | Detect runaway folders | `twin/fs_graph.rs` | `[N]` | Twin: growth detection |
| 107 | Detect recursive duplication | `engines/duplicate/engine.rs` | `[N]` | Duplicate engine |
| 108 | Detect unnecessary copies | `engines/duplicate/engine.rs` | `[N]` | Duplicate engine |
| 109 | Detect stale projects | `twin/fs_graph.rs` | `[N]` | Twin: stale project detection |
| 110 | Detect inactive datasets | `twin/fs_graph.rs` | `[N]` | Twin: inactive dataset detection |
| 111 | Detect unused assets | `twin/fs_graph.rs` | `[N]` | Twin: unused asset detection |
| 112 | Detect build artifacts | `engines/clean/rules.rs` | `[E]` | BuildArtifact category |
| 113 | Detect cache regeneration ability | `engines/clean/rules.rs` | `[N]` | Regen prediction |
| 114 | Build filesystem knowledge graph | `twin/fs_graph.rs` | `[N]` | Twin: FS knowledge graph |
| 115 | Track filesystem evolution | `twin/fs_graph.rs` | `[N]` | Twin: evolution tracking |
| 116 | Predict storage growth | `twin/fs_graph.rs` | `[N]` | Twin: growth prediction |
| 117 | Forecast storage exhaustion | `twin/fs_graph.rs` | `[N]` | Twin: exhaustion forecast |
| 118 | Recommend cleanup | `intelligence/advisor.rs` | `[E]` | Advisor recommendations |
| 119 | Simulate cleanup impact | `twin/reasoning.rs` | `[N]` | Twin: simulation |
| 120 | Execute reversible cleanup | `cleanup/transaction.rs` | `[E]` | Undo support |

### 14. Process Intelligence System (121–160)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 121 | Observe every running process | `twin/process.rs` | `[N]` | Twin: process observation |
| 122 | Identify process owner | `twin/process.rs` | `[N]` | Twin: owner identification |
| 123 | Identify parent process | `twin/process.rs` | `[N]` | Twin: parent tracking |
| 124 | Identify child processes | `twin/process.rs` | `[N]` | Twin: child tracking |
| 125 | Map process trees | `twin/process.rs` | `[N]` | Twin: process tree |
| 126 | Map application → processes | `twin/process.rs` | `[N]` | Twin: app-process mapping |
| 127 | Track CPU usage | `engines/optimize/telemetry.rs` | `[E]` | CPU tracking |
| 128 | Track GPU usage | `twin/process.rs` | `[N]` | Twin: GPU tracking |
| 129 | Track memory usage | `engines/optimize/telemetry.rs` | `[E]` | Memory tracking |
| 130 | Track memory allocations | `twin/process.rs` | `[N]` | Twin: allocation tracking |
| 131 | Track disk reads | `twin/process.rs` | `[N]` | Twin: disk read tracking |
| 132 | Track disk writes | `twin/process.rs` | `[N]` | Twin: disk write tracking |
| 133 | Track network traffic | `twin/process.rs` | `[N]` | Twin: network tracking |
| 134 | Track energy impact | `twin/process.rs` | `[N]` | Twin: energy tracking |
| 135 | Track wakeups | `twin/process.rs` | `[N]` | Twin: wakeup tracking |
| 136 | Detect idle processes | `twin/process.rs` | `[N]` | Twin: idle detection |
| 137 | Detect runaway processes | `intelligence/advisor.rs` | `[E]` | Runaway detection |
| 138 | Detect memory leaks | `twin/process.rs` | `[N]` | Twin: leak detection |
| 139 | Detect CPU spikes | `twin/process.rs` | `[N]` | Twin: spike detection |
| 140 | Detect abnormal behavior | `twin/process.rs` | `[N]` | Twin: anomaly detection |
| 141 | Detect crashed services | `twin/process.rs` | `[N]` | Twin: crash detection |
| 142 | Predict application failure | `twin/process.rs` | `[N]` | Twin: failure prediction |
| 143 | Recommend process termination | `intelligence/advisor.rs` | `[E]` | Advisor recommendations |
| 144 | Restart failed services | `engines/startup/engine.rs` | `[N]` | Service restart |
| 145 | Analyze application efficiency | `twin/process.rs` | `[N]` | Twin: efficiency analysis |
| 146 | Compare application versions | `twin/process.rs` | `[N]` | Twin: version comparison |
| 147 | Create performance profiles | `twin/process.rs` | `[N]` | Twin: performance profiles |
| 148 | Create workload fingerprints | `twin/process.rs` | `[N]` | Twin: workload fingerprints |
| 149 | Learn normal behavior | `twin/process.rs` | `[N]` | Twin: behavior learning |
| 150 | Detect anomalies | `twin/process.rs` | `[N]` | Twin: anomaly detection |
| 151 | Explain resource usage | `twin/process.rs` | `[N]` | Twin: usage explanation |
| 152 | Predict bottlenecks | `twin/process.rs` | `[N]` | Twin: bottleneck prediction |
| 153 | Optimize scheduling | `twin/process.rs` | `[N]` | Twin: scheduling optimization |
| 154 | Optimize background activity | `twin/process.rs` | `[N]` | Twin: background optimization |
| 155 | Prioritize active applications | `twin/process.rs` | `[N]` | Twin: app prioritization |
| 156 | Reduce unnecessary work | `twin/process.rs` | `[N]` | Twin: work reduction |
| 157 | Balance workloads | `twin/process.rs` | `[N]` | Twin: workload balancing |
| 158 | Maintain process history | `twin/process.rs` | `[N]` | Twin: history tracking |
| 159 | Generate performance reports | `cli/output.rs` | `[X]` | Performance report format |
| 160 | Build process intelligence graph | `twin/process.rs` | `[N]` | Twin: process graph |

### 15. Unified Memory Intelligence (161–200)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 161 | Monitor memory pressure | `intelligence/system_awareness.rs` | `[E]` | MemoryDimension |
| 162 | Monitor compressed memory | `intelligence/system_awareness.rs` | `[E]` | compressed_bytes |
| 163 | Monitor swap usage | `intelligence/system_awareness.rs` | `[E]` | swap_used_bytes |
| 164 | Monitor purgeable memory | `engines/optimize/telemetry.rs` | `[X]` | Add purgeable tracking |
| 165 | Monitor application memory footprint | `twin/memory.rs` | `[N]` | Twin: footprint tracking |
| 166 | Track memory allocation patterns | `twin/memory.rs` | `[N]` | Twin: allocation patterns |
| 167 | Track memory leaks | `twin/memory.rs` | `[N]` | Twin: leak tracking |
| 168 | Detect memory fragmentation | `twin/memory.rs` | `[N]` | Twin: fragmentation detection |
| 169 | Identify memory-heavy applications | `engines/optimize/telemetry.rs` | `[E]` | Top processes |
| 170 | Identify inefficient applications | `twin/memory.rs` | `[N]` | Twin: inefficiency detection |
| 171 | Predict memory exhaustion | `twin/memory.rs` | `[N]` | Twin: exhaustion prediction |
| 172 | Predict swap events | `twin/memory.rs` | `[N]` | Twin: swap prediction |
| 173 | Recommend application closure | `intelligence/advisor.rs` | `[E]` | Advisor recommendations |
| 174 | Recommend workload migration | `twin/memory.rs` | `[N]` | Twin: migration recs |
| 175 | Optimize application ordering | `twin/memory.rs` | `[N]` | Twin: app ordering |
| 176 | Optimize background processes | `twin/memory.rs` | `[N]` | Twin: background optimization |
| 177 | Manage ML model memory | `twin/memory.rs` | `[N]` | Twin: ML memory management |
| 178 | Manage GPU memory usage | `twin/memory.rs` | `[N]` | Twin: GPU memory |
| 179 | Manage Metal allocations | `twin/memory.rs` | `[N]` | Twin: Metal allocations |
| 180 | Manage Neural Engine workloads | `twin/memory.rs` | `[N]` | Twin: ANE workloads |
| 181 | Detect memory contention | `twin/memory.rs` | `[N]` | Twin: contention detection |
| 182 | Understand Apple Silicon unified memory | `twin/memory.rs` | `[N]` | Twin: unified memory model |
| 183 | Build memory topology model | `twin/memory.rs` | `[N]` | Twin: memory topology |
| 184 | Create memory pressure forecast | `twin/memory.rs` | `[N]` | Twin: pressure forecast |
| 185 | Analyze memory over time | `twin/memory.rs` | `[N]` | Twin: temporal analysis |
| 186 | Compare workloads | `twin/memory.rs` | `[N]` | Twin: workload comparison |
| 187 | Learn user workflows | `twin/memory.rs` | `[N]` | Twin: workflow learning |
| 188 | Predict next applications | `twin/memory.rs` | `[N]` | Twin: app prediction |
| 189 | Pre-stage resources | `twin/memory.rs` | `[N]` | Twin: resource pre-staging |
| 190 | Release unused resources | `twin/memory.rs` | `[N]` | Twin: resource release |
| 191 | Optimize developer workloads | `twin/memory.rs` | `[N]` | Twin: dev workload optimization |
| 192 | Optimize creative workloads | `twin/memory.rs` | `[N]` | Twin: creative workload optimization |
| 193 | Optimize gaming workloads | `twin/memory.rs` | `[N]` | Twin: gaming workload optimization |
| 194 | Optimize AI workloads | `twin/memory.rs` | `[N]` | Twin: AI workload optimization |
| 195 | Optimize virtualization workloads | `twin/memory.rs` | `[N]` | Twin: VM workload optimization |
| 196 | Explain memory decisions | `twin/memory.rs` | `[N]` | Twin: decision explanation |
| 197 | Simulate memory changes | `twin/memory.rs` | `[N]` | Twin: memory simulation |
| 198 | Automate memory policies | `twin/memory.rs` | `[N]` | Twin: policy automation |
| 199 | Maintain memory history | `twin/memory.rs` | `[N]` | Twin: memory history |
| 200 | Create memory intelligence layer | `twin/memory.rs` | `[N]` | Twin: memory intelligence |

### 16. Energy & Battery Twin (201–235)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 201 | Model battery behavior | `twin/energy.rs` | `[N]` | Twin: battery model |
| 202 | Track charging patterns | `twin/energy.rs` | `[N]` | Twin: charging tracking |
| 203 | Track discharge patterns | `twin/energy.rs` | `[N]` | Twin: discharge tracking |
| 204 | Track energy impact | `twin/energy.rs` | `[N]` | Twin: energy impact |
| 205 | Identify battery-heavy apps | `twin/energy.rs` | `[N]` | Twin: battery-heavy detection |
| 206 | Identify background drain | `twin/energy.rs` | `[N]` | Twin: drain detection |
| 207 | Identify network drain | `twin/energy.rs` | `[N]` | Twin: network drain |
| 208 | Identify display drain | `twin/energy.rs` | `[N]` | Twin: display drain |
| 209 | Predict battery life | `twin/energy.rs` | `[N]` | Twin: battery life prediction |
| 210 | Optimize energy usage | `twin/energy.rs` | `[N]` | Twin: energy optimization |
| 211 | Adjust background activity | `twin/energy.rs` | `[N]` | Twin: background adjustment |
| 212 | Recommend power modes | `twin/energy.rs` | `[N]` | Twin: power mode recs |
| 213 | Detect abnormal battery aging | `twin/energy.rs` | `[N]` | Twin: aging detection |
| 214 | Analyze thermal efficiency | `twin/energy.rs` | `[N]` | Twin: thermal efficiency |
| 215 | Analyze charging efficiency | `twin/energy.rs` | `[N]` | Twin: charging efficiency |
| 216 | Predict battery degradation | `twin/energy.rs` | `[N]` | Twin: degradation prediction |
| 217 | Model user mobility | `twin/energy.rs` | `[N]` | Twin: mobility model |
| 218 | Learn charging habits | `twin/energy.rs` | `[N]` | Twin: habit learning |
| 219 | Recommend charging strategy | `twin/energy.rs` | `[N]` | Twin: charging strategy |
| 220 | Detect inefficient workflows | `twin/energy.rs` | `[N]` | Twin: inefficient workflow detection |
| 221 | Compare battery sessions | `twin/energy.rs` | `[N]` | Twin: session comparison |
| 222 | Generate battery reports | `twin/energy.rs` | `[N]` | Twin: battery reports |
| 223 | Forecast battery health | `twin/energy.rs` | `[N]` | Twin: health forecast |
| 224 | Detect hardware problems | `twin/energy.rs` | `[N]` | Twin: hardware problem detection |
| 225 | Analyze sleep efficiency | `twin/energy.rs` | `[N]` | Twin: sleep analysis |
| 226 | Analyze wake causes | `twin/energy.rs` | `[N]` | Twin: wake analysis |
| 227 | Optimize standby | `twin/energy.rs` | `[N]` | Twin: standby optimization |
| 228 | Reduce idle power | `twin/energy.rs` | `[N]` | Twin: idle power reduction |
| 229 | Manage energy priorities | `twin/energy.rs` | `[N]` | Twin: energy priorities |
| 230 | Create energy profile | `twin/energy.rs` | `[N]` | Twin: energy profile |
| 231 | Predict future battery state | `twin/energy.rs` | `[N]` | Twin: state prediction |
| 232 | Explain energy consumption | `twin/energy.rs` | `[N]` | Twin: consumption explanation |
| 233 | Simulate power changes | `twin/energy.rs` | `[N]` | Twin: power simulation |
| 234 | Automate energy optimization | `twin/energy.rs` | `[N]` | Twin: energy automation |
| 235 | Maintain energy history | `twin/energy.rs` | `[N]` | Twin: energy history |

### 17. Application Intelligence Agent (236–275)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 236 | Understand application purpose | `twin/app_agent.rs` | `[N]` | Twin: app purpose |
| 237 | Understand application behavior | `twin/app_agent.rs` | `[N]` | Twin: app behavior |
| 238 | Understand application dependencies | `twin/app_agent.rs` | `[N]` | Twin: app dependencies |
| 239 | Understand application files | `twin/app_agent.rs` | `[N]` | Twin: app files |
| 240 | Understand application permissions | `twin/app_agent.rs` | `[N]` | Twin: app permissions |
| 241 | Detect unused applications | `engines/envmap/apps.rs` | `[N]` | Unused app detection |
| 242 | Detect duplicate applications | `engines/envmap/apps.rs` | `[N]` | Duplicate app detection |
| 243 | Detect abandoned applications | `engines/envmap/apps.rs` | `[N]` | Abandoned app detection |
| 244 | Detect broken installations | `engines/envmap/discovery.rs` | `[N]` | Broken install detection |
| 245 | Detect corrupted preferences | `twin/app_agent.rs` | `[N]` | Twin: corrupted prefs |
| 246 | Predict uninstall impact | `twin/app_agent.rs` | `[N]` | Twin: uninstall impact |
| 247 | Perform intelligent uninstall | `cleanup/transaction.rs` | `[X]` | App uninstall |
| 248 | Restore removed applications | `cleanup/undo.rs` | `[E]` | Undo support |
| 249 | Track application evolution | `twin/app_agent.rs` | `[N]` | Twin: app evolution |
| 250 | Recommend alternatives | `twin/app_agent.rs` | `[N]` | Twin: alternative recs |
| 251 | Detect inefficient apps | `twin/app_agent.rs` | `[N]` | Twin: inefficiency detection |
| 252 | Benchmark applications | `twin/app_agent.rs` | `[N]` | Twin: app benchmarking |
| 253 | Compare applications | `twin/app_agent.rs` | `[N]` | Twin: app comparison |
| 254 | Predict crashes | `twin/app_agent.rs` | `[N]` | Twin: crash prediction |
| 255 | Diagnose failures | `twin/app_agent.rs` | `[N]` | Twin: failure diagnosis |
| 256 | Explain application problems | `twin/app_agent.rs` | `[N]` | Twin: problem explanation |
| 257 | Recommend fixes | `twin/app_agent.rs` | `[N]` | Twin: fix recommendations |
| 258 | Automatically repair issues | `twin/app_agent.rs` | `[N]` | Twin: auto repair |
| 259 | Maintain application profiles | `twin/app_agent.rs` | `[N]` | Twin: app profiles |
| 260 | Build application knowledge graph | `twin/app_agent.rs` | `[N]` | Twin: app knowledge graph |
| 261 | Predict user needs | `twin/app_agent.rs` | `[N]` | Twin: need prediction |
| 262 | Preload common tools | `twin/app_agent.rs` | `[N]` | Twin: tool preloading |
| 263 | Optimize workflows | `twin/app_agent.rs` | `[N]` | Twin: workflow optimization |
| 264 | Automate repetitive actions | `twin/app_agent.rs` | `[N]` | Twin: action automation |
| 265 | Create application policies | `twin/app_agent.rs` | `[N]` | Twin: app policies |
| 266 | Manage permissions | `engines/privacy/engine.rs` | `[N]` | Privacy engine |
| 267 | Audit security posture | `engines/privacy/engine.rs` | `[N]` | Privacy engine |
| 268 | Detect suspicious behavior | `twin/app_agent.rs` | `[N]` | Twin: suspicious behavior |
| 269 | Detect unnecessary network access | `twin/app_agent.rs` | `[N]` | Twin: network access detection |
| 270 | Detect excessive background activity | `twin/app_agent.rs` | `[N]` | Twin: background activity |
| 271 | Generate app health score | `twin/app_agent.rs` | `[N]` | Twin: app health score |
| 272 | Provide recommendations | `intelligence/advisor.rs` | `[E]` | Advisor |
| 273 | Explain recommendations | `intelligence/advisor.rs` | `[E]` | NL explanations |
| 274 | Simulate changes | `twin/reasoning.rs` | `[N]` | Twin: change simulation |
| 275 | Apply safe changes | `cleanup/transaction.rs` | `[E]` | Safe cleanup |

### 18. AI Reasoning Layer (276–330)

| # | Operation | Target | Status | Notes |
|---|-----------|--------|--------|-------|
| 276 | Build complete Mac knowledge graph | `twin/reasoning.rs` | `[N]` | Twin: knowledge graph |
| 277 | Maintain historical snapshots | `twin/reasoning.rs` | `[N]` | Twin: historical snapshots |
| 278 | Compare system states | `intelligence/zen.rs` | `[E]` | Before/after comparison |
| 279 | Detect regressions | `twin/reasoning.rs` | `[N]` | Twin: regression detection |
| 280 | Predict future problems | `twin/reasoning.rs` | `[N]` | Twin: problem prediction |
| 281 | Recommend preventive actions | `twin/reasoning.rs` | `[N]` | Twin: preventive recs |
| 282 | Explain system behavior | `twin/reasoning.rs` | `[N]` | Twin: behavior explanation |
| 283 | Answer "why is my Mac slow?" | `twin/reasoning.rs` | `[N]` | Twin: causal analysis |
| 284 | Answer "what changed?" | `twin/reasoning.rs` | `[N]` | Twin: change tracking |
| 285 | Answer "what caused this?" | `twin/reasoning.rs` | `[N]` | Twin: causal inference |
| 286 | Simulate optimization outcomes | `twin/reasoning.rs` | `[N]` | Twin: outcome simulation |
| 287 | Create optimization plans | `twin/reasoning.rs` | `[N]` | Twin: optimization plans |
| 288 | Rank improvements by impact | `intelligence/advisor.rs` | `[X]` | Impact ranking |
| 289 | Estimate risk | `twin/reasoning.rs` | `[N]` | Twin: risk estimation |
| 290 | Roll back changes | `cleanup/undo.rs` | `[E]` | Undo support |
| 291 | Learn user preferences | `intelligence/advisor.rs` | `[E]` | AdaptiveState |
| 292 | Learn workflows | `twin/reasoning.rs` | `[N]` | Twin: workflow learning |
| 293 | Learn acceptable tradeoffs | `twin/reasoning.rs` | `[N]` | Twin: tradeoff learning |
| 294 | Build personalized policies | `twin/reasoning.rs` | `[N]` | Twin: personalized policies |
| 295 | Automate maintenance | `intelligence/daemon.rs` | `[E]` | Daemon automation |
| 296 | Detect unusual behavior | `twin/reasoning.rs` | `[N]` | Twin: unusual behavior |
| 297 | Detect emerging failures | `twin/reasoning.rs` | `[N]` | Twin: failure detection |
| 298 | Predict SSD wear | `twin/reasoning.rs` | `[N]` | Twin: SSD wear prediction |
| 299 | Predict battery decline | `twin/energy.rs` | `[N]` | Twin: battery decline |
| 300 | Predict storage problems | `twin/fs_graph.rs` | `[N]` | Twin: storage prediction |
| 301 | Predict performance degradation | `twin/reasoning.rs` | `[N]` | Twin: degradation prediction |
| 302 | Recommend hardware upgrades | `twin/reasoning.rs` | `[N]` | Twin: upgrade recs |
| 303 | Recommend software changes | `twin/reasoning.rs` | `[N]` | Twin: software change recs |
| 304 | Explain every action | `intelligence/advisor.rs` | `[E]` | NL explanations |
| 305 | Maintain trust score | `twin/reasoning.rs` | `[N]` | Twin: trust score |
| 306 | Create system health score | `intelligence/system_awareness.rs` | `[E]` | health_score |
| 307 | Create digital twin visualization | `gui/XMacApp/` | `[N]` | New: twin visualization view |
| 308 | Expose APIs | `cli/output.rs` | `[X]` | API output format |
| 309 | Allow AI agents to query state | `twin/reasoning.rs` | `[N]` | Twin: agent query API |
| 310 | Allow autonomous optimization | `twin/reasoning.rs` | `[N]` | Twin: autonomous optimization |
| 311 | Sandbox proposed changes | `twin/reasoning.rs` | `[N]` | Twin: change sandboxing |
| 312 | Test before execution | `cleanup/preflight.rs` | `[E]` | Preflight checks |
| 313 | Monitor after execution | `cleanup/verification.rs` | `[E]` | Post-cleanup verification |
| 314 | Verify improvements | `cleanup/verification.rs` | `[E]` | Verification |
| 315 | Learn from outcomes | `intelligence/advisor.rs` | `[E]` | AdaptiveState learning |
| 316 | Improve optimization models | `gnn/train.py` | `[X]` | GNN improvement |
| 317 | Share anonymized benchmarks | `twin/reasoning.rs` | `[N]` | Twin: benchmark sharing |
| 318 | Compare against similar Macs | `twin/reasoning.rs` | `[N]` | Twin: Mac comparison |
| 319 | Detect unique problems | `twin/reasoning.rs` | `[N]` | Twin: unique problem detection |
| 320 | Become a continuous Mac OS layer | `twin/reasoning.rs` | `[N]` | Twin: OS layer |

---

## Summary Statistics

| Status | Count | Percentage |
|--------|-------|------------|
| `[E]` Existing | 82 | 13% |
| `[X]` Extend | 78 | 12.4% |
| `[N]` New | 470 | 74.6% |
| **Total** | **630** | 100% |

| Target Module | New Ops |
|---------------|---------|
| `twin/hardware.rs` | 35 |
| `twin/software_genome.rs` | 35 |
| `twin/fs_graph.rs` | 35 |
| `twin/process.rs` | 40 |
| `twin/memory.rs` | 40 |
| `twin/energy.rs` | 35 |
| `twin/app_agent.rs` | 40 |
| `twin/reasoning.rs` | 45 |
| `engines/duplicate/` | 30 |
| `engines/startup/` | 30 |
| `engines/privacy/` | 30 |
| **Total New** | **395** |
