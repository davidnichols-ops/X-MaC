//! Post-scan remediation script generator.
//!
//! Given the findings collected during a scan, produces a self-contained,
//! reviewable shell script (`xmac-fixes.sh`) that the user can inspect and
//! execute to apply fixes. By policy every destructive command is emitted
//! **commented out** so the user must explicitly opt in. Non-destructive,
//! reversible commands (e.g. `chmod o-w` on world-writable files) are emitted
//! active but guarded by a top-level confirmation prompt.
//!
//! The generator is deliberately conservative: findings from categories that
//! are known to be false-positive prone (broken symlinks with relative
//! targets, orphan detection, missing dylibs) are placed in a separate
//! "REVIEW CAREFULLY" section and never emitted as active commands.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::core::types::{Category, Finding, Severity, Target};

pub struct FixScriptGenerator {
    /// Where the generated script will be written.
    out_path: PathBuf,
}

impl FixScriptGenerator {
    pub fn new(out_path: PathBuf) -> Self {
        Self { out_path }
    }

    /// Build the remediation script body from a slice of findings.
    pub fn build_script(findings: &[Finding]) -> String {
        let mut s = String::new();

        s.push_str("#!/bin/bash\n");
        s.push_str("#\n");
        s.push_str("# X-MaC remediation script\n");
        s.push_str("# Generated from a completed X-MaC scan.\n");
        s.push_str("#\n");
        s.push_str("# IMPORTANT\n");
        s.push_str("#   This script is READ-ONLY by default. Every destructive command is\n");
        s.push_str("#   commented out and must be uncommented after you have reviewed it.\n");
        s.push_str("#   Non-destructive commands run only after you confirm below.\n");
        s.push_str("#\n");
        s.push_str("# Review every line before running. You are responsible for the\n");
        s.push_str("# consequences of executing this script.\n");
        s.push_str("#\n");
        s.push_str(&format!("# Findings covered: {}\n", findings.len()));
        s.push_str("\n");

        // Confirmation gate for the active (non-destructive) section.
        s.push_str("set -euo pipefail\n\n");
        s.push_str("if [ \"${1:-}\" != \"--yes\" ]; then\n");
        s.push_str("  echo \"This script applies non-destructive fixes from an X-MaC scan.\"\n");
        s.push_str("  echo \"Destructive fixes are commented out and must be enabled manually.\"\n");
        s.push_str("  echo \"Review this file first, then re-run with: $0 --yes\"\n");
        s.push_str("  exit 1\n");
        s.push_str("fi\n\n");

        let (safe, review) = Self::partition(findings);

        Self::write_summary(&mut s, findings, &safe, &review);

        // ---- Active, non-destructive fixes ----
        s.push_str("# ====================================================================\n");
        s.push_str("# Section 1: Non-destructive fixes (active, run after --yes)\n");
        s.push_str("# ====================================================================\n\n");
        Self::write_safe_section(&mut s, &safe);

        // ---- Destructive / review-required fixes ----
        s.push_str("\n# ====================================================================\n");
        s.push_str("# Section 2: Destructive / review-required fixes (COMMENTED OUT)\n");
        s.push_str("# Uncomment individual lines after verifying they are correct.\n");
        s.push_str("# ====================================================================\n\n");
        Self::write_review_section(&mut s, &review);

        s.push_str("\n# End of X-MaC remediation script\n");
        s
    }

    /// Write the generator's script to disk and return the path written.
    pub fn write(&self, findings: &[Finding]) -> std::io::Result<PathBuf> {
        let body = Self::build_script(findings);
        let mut file = std::fs::File::create(&self.out_path)?;
        file.write_all(body.as_bytes())?;

        // Best-effort chmod +x; ignore failure (e.g. on filesystems that
        // don't support the permission bit).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.out_path, std::fs::Permissions::from_mode(0o755));
        }

        Ok(self.out_path.clone())
    }

    // -- internals ---------------------------------------------------------

    /// Split findings into "safe to auto-apply" and "needs manual review".
    fn partition(findings: &[Finding]) -> (Vec<&Finding>, Vec<&Finding>) {
        let mut safe = Vec::new();
        let mut review = Vec::new();

        for f in findings {
            match f.category {
                // World-writable permission fixes are reversible and safe.
                Category::PermissionIssue if Self::is_world_writable(f) => safe.push(f),
                // Everything else requires a human in the loop.
                Category::PermissionIssue => review.push(f),
                Category::BrokenSymlink
                | Category::MissingDylib
                | Category::OrphanFile
                | Category::DuplicateFile
                | Category::Cache
                | Category::Log
                | Category::XcodeArtifact
                | Category::TempFile
                | Category::BuildArtifact
                | Category::PackageManagerCache
                | Category::BrowserCache
                | Category::MailAttachment
                | Category::IosBackup
                | Category::LanguageFile
                | Category::UniversalBinary
                | Category::TrashBin
                | Category::DocumentVersion
                | Category::PortConflict
                | Category::PathConflict
                | Category::EnvVarConflict
                | Category::ShellConflict
                | Category::InvalidSignature => review.push(f),
                // System maintenance tasks are informational — the command
                // is in the remediation hint.
                Category::SystemMaintenance => review.push(f),
                // Pure discovery / informational — no fix to apply.
                Category::PythonEnv
                | Category::NodeEnv
                | Category::ContainerRuntime
                | Category::PackageManager
                | Category::InstalledApp
                | Category::SystemInfo
                | Category::LargeFile => {}
            }
        }

        (safe, review)
    }

    fn is_world_writable(f: &Finding) -> bool {
        f.title.contains("World-writable")
    }

    fn write_summary(s: &mut String, all: &[Finding], safe: &[&Finding], review: &[&Finding]) {
        let reclaimable: u64 = all.iter().filter_map(|f| f.size_bytes).sum();
        let by_severity = |sev: Severity| all.iter().filter(|f| f.severity == sev).count();

        s.push_str("# ------------------------------------------------------------------\n");
        s.push_str("# Summary\n");
        s.push_str("# ------------------------------------------------------------------\n");
        s.push_str(&format!("#   Total findings:        {}\n", all.len()));
        s.push_str(&format!("#   Reclaimable bytes:      {} ({} bytes)\n",
            crate::util::disk::format_bytes(reclaimable), reclaimable));
        s.push_str(&format!("#   Critical / High / Med:  {} / {} / {}\n",
            by_severity(Severity::Critical), by_severity(Severity::High), by_severity(Severity::Medium)));
        s.push_str(&format!("#   Low / Info:             {} / {}\n",
            by_severity(Severity::Low), by_severity(Severity::Info)));
        s.push_str(&format!("#   Active fixes:           {}\n", safe.len()));
        s.push_str(&format!("#   Review-required fixes:  {}\n", review.len()));
        s.push_str("\n");
    }

    fn write_safe_section(s: &mut String, safe: &[&Finding]) {
        if safe.is_empty() {
            s.push_str("# (no non-destructive fixes were derived from this scan)\n");
            return;
        }

        // Group world-writable fixes; emit chmod o-w per path.
        let mut emitted = 0;
        for f in safe {
            if let Target::Path(ref p) = f.target {
                let path = shell_quote(p);
                s.push_str(&format!("# {} (perms {:?})\n", f.title, f.severity));
                s.push_str(&format!("chmod o-w {}\n", path));
                emitted += 1;
            }
        }
        if emitted == 0 {
            s.push_str("# (no path-targeted non-destructive fixes)\n");
        }
    }

    fn write_review_section(s: &mut String, review: &[&Finding]) {
        // Group by category for readability.
        let mut groups: Vec<(Category, Vec<&Finding>)> = Vec::new();
        for f in review {
            if let Some(slot) = groups.iter_mut().find(|(c, _)| *c == f.category) {
                slot.1.push(f);
            } else {
                groups.push((f.category, vec![f]));
            }
        }

        if groups.is_empty() {
            s.push_str("# (no review-required fixes)\n");
            return;
        }

        for (category, items) in groups {
            s.push_str(&format!("# --- {} ({} finding(s)) ---\n", Self::category_label(category), items.len()));
            s.push_str(&format!("# {}\n", Self::category_warning(category)));

            for f in items {
                Self::write_review_finding(s, f);
            }
            s.push_str("\n");
        }
    }

    fn write_review_finding(s: &mut String, f: &Finding) {
        let target_str = match f.target {
            Target::Path(ref p) => shell_quote(p),
            Target::Port(port) => format!("port:{}", port),
            Target::Process(pid) => format!("pid:{}", pid),
            Target::EnvironmentVariable(ref v) => format!("env:{}", v),
            Target::LaunchdLabel(ref l) => format!("label:{}", l),
            Target::Package(ref p) => format!("pkg:{}", p),
        };

        // One-line context so the reviewer can find the finding in the report.
        s.push_str(&format!("# [{}] {}\n", severity_label(f.severity), f.title));
        s.push_str(&format!("#   target: {}\n", target_str));

        if let Some(hint) = &f.remediation_hint {
            s.push_str(&format!("#   hint:   {}\n", hint));
        }

        // Emit the suggested command, commented out.
        match f.category {
            Category::BrokenSymlink => {
                if let Target::Path(ref p) = f.target {
                    s.push_str(&format!("# rm -- {}\n", shell_quote(p)));
                }
            }
            Category::Cache | Category::Log | Category::XcodeArtifact | Category::OrphanFile
            | Category::TempFile | Category::BuildArtifact | Category::PackageManagerCache
            | Category::BrowserCache | Category::MailAttachment | Category::IosBackup
            | Category::LanguageFile | Category::TrashBin | Category::DocumentVersion
            | Category::UniversalBinary => {
                if let Target::Path(ref p) = f.target {
                    // Use `du -sh` first so the reviewer sees the size, then rm.
                    s.push_str(&format!("# du -sh {}\n", shell_quote(p)));
                    s.push_str(&format!("# rm -rf -- {}\n", shell_quote(p)));
                }
            }
            Category::SystemMaintenance => {
                // The remediation hint contains the command to run.
                if let Some(hint) = &f.remediation_hint {
                    s.push_str(&format!("# {}\n", hint));
                }
            }
            Category::DuplicateFile => {
                if let Some(paths) = f.metadata.get("duplicate_paths").and_then(|v| v.as_array()) {
                    s.push_str("# duplicate set:\n");
                    for path in paths {
                        if let Some(ps) = path.as_str() {
                            s.push_str(&format!("#   du -sh -- {}\n", shell_quote(&PathBuf::from(ps))));
                        }
                    }
                    s.push_str("# remove the redundant copies (keep one):\n");
                    for path in paths {
                        if let Some(ps) = path.as_str() {
                            s.push_str(&format!("# rm -- {}\n", shell_quote(&PathBuf::from(ps))));
                        }
                    }
                }
            }
            Category::PortConflict => {
                if let Some(pid) = f.metadata.get("pid").and_then(|v| v.as_u64()) {
                    s.push_str(&format!("# kill {}\n", pid));
                }
            }
            Category::MissingDylib => {
                if let Target::Path(ref p) = f.target {
                    s.push_str(&format!("# otool -L {}\n", shell_quote(p)));
                    s.push_str("# # then: brew reinstall <owning formula> or rebuild the library\n");
                }
            }
            Category::PermissionIssue => {
                // SUID/SGID landed here (world-writable went to the safe section).
                if let Target::Path(ref p) = f.target {
                    if f.title.contains("SUID") {
                        s.push_str(&format!("# chmod u-s {}\n", shell_quote(p)));
                    } else if f.title.contains("SGID") {
                        s.push_str(&format!("# chmod g-s {}\n", shell_quote(p)));
                    }
                }
            }
            Category::PathConflict | Category::EnvVarConflict | Category::ShellConflict => {
                // Informational — no command, the hint above is the guidance.
                s.push_str("# (informational — edit shell/PATH config manually)\n");
            }
            Category::InvalidSignature => {
                if let Target::Path(ref p) = f.target {
                    s.push_str(&format!("# codesign -dv {}\n", shell_quote(p)));
                    s.push_str("# # then reinstall the owning package if the signature is broken\n");
                }
            }
            // Discovery categories are filtered out before reaching here.
            Category::PythonEnv
            | Category::NodeEnv
            | Category::ContainerRuntime
            | Category::PackageManager
            | Category::InstalledApp
            | Category::SystemInfo
            | Category::LargeFile => {
                s.push_str("# (informational)\n");
            }
        }
        s.push_str("\n");
    }

    fn category_label(c: Category) -> &'static str {
        match c {
            Category::Cache => "Cache files",
            Category::Log => "Log files",
            Category::XcodeArtifact => "Xcode artifacts",
            Category::OrphanFile => "Orphaned app-support directories",
            Category::DuplicateFile => "Duplicate files",
            Category::PathConflict => "PATH conflicts",
            Category::EnvVarConflict => "Environment variable conflicts",
            Category::PortConflict => "Port conflicts",
            Category::ShellConflict => "Shell conflicts",
            Category::PermissionIssue => "Permission issues (SUID/SGID)",
            Category::BrokenSymlink => "Broken symlinks",
            Category::MissingDylib => "Missing dylib dependencies",
            Category::InvalidSignature => "Invalid code signatures",
            Category::TempFile => "Temp files",
            Category::BuildArtifact => "Build artifacts",
            Category::PackageManagerCache => "Package-manager caches",
            Category::BrowserCache => "Browser caches",
            Category::MailAttachment => "Mail attachments",
            Category::IosBackup => "iOS device backups",
            Category::LanguageFile => "Language files",
            Category::UniversalBinary => "Universal binaries",
            Category::LargeFile => "Large files",
            Category::TrashBin => "Trash bins",
            Category::DocumentVersion => "Document versions",
            Category::SystemMaintenance => "System maintenance tasks",
            _ => "Other",
        }
    }

    fn category_warning(c: Category) -> &'static str {
        match c {
            Category::BrokenSymlink => "WARNING: relative symlinks are often misreported as broken. Verify with `readlink -f <path>` before removing.",
            Category::OrphanFile => "WARNING: orphan detection assumes the support dir name matches the .app bundle name, which is rarely true. Verify the app is really uninstalled before deleting.",
            Category::MissingDylib => "WARNING: missing-dylib detection parses `otool -L` output and can misread version annotations as part of the path. Verify with `otool -L <path>` before reinstalling.",
            Category::DuplicateFile => "Keep one copy of each duplicate set; deleting all copies loses data.",
            Category::Cache | Category::Log => "Safe to delete, but apps will rebuild these caches.",
            Category::XcodeArtifact => "Safe to delete; Xcode rebuilds DerivedData on next build.",
            Category::TempFile => "Safe to delete. .DS_Store files are recreated by Finder; swap files are from editor crashes.",
            Category::BuildArtifact => "Safe to delete. Regenerated by the build tool on next compile/install (e.g. npm install, cargo build).",
            Category::PackageManagerCache => "Safe to delete. Regenerated on next package install (e.g. npm install, pip install, cargo build).",
            Category::BrowserCache => "Safe to delete. Browser will rebuild cache on next use.",
            Category::MailAttachment => "Attachments may be re-downloaded from server. Review before deleting.",
            Category::IosBackup => "Keep the most recent backup per device. Old backups can be removed if no longer needed.",
            Category::LanguageFile => "Safe to remove non-English .lproj dirs. They are restored on app update.",
            Category::UniversalBinary => "Thinning removes unused architecture slices. Use lipo -extract. App signatures are preserved.",
            Category::TrashBin => "Empty trash to reclaim space. Use 'rm -rf' for locked files that won't empty normally.",
            Category::DocumentVersion => "macOS document version history. Removing frees space but loses revision history.",
            Category::SystemMaintenance => "System maintenance commands. Some require sudo. Review each command before running.",
            Category::PortConflict => "Killing a process may interrupt active work. Confirm the PID is the one you want to stop.",
            Category::PermissionIssue => "Removing SUID/SGID bits may break binaries that legitimately need elevated privileges.",
            _ => "Review before applying.",
        }
    }
}

// -- helpers ------------------------------------------------------------------

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}

/// Single-quote a path for safe inclusion in a shell command.
fn shell_quote(p: &Path) -> String {
    let s = p.to_string_lossy();
    // Replace any embedded single quote with '\'' and wrap in single quotes.
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{EngineId, Finding, Severity, Target};
    use std::path::PathBuf;

    fn make_finding(category: Category, severity: Severity, title: &str, target: Target) -> Finding {
        Finding::new(EngineId::Clean, severity, category, target, title, "desc")
    }

    #[test]
    fn test_shell_quote_plain_path() {
        assert_eq!(shell_quote(&PathBuf::from("/tmp/foo")), "'/tmp/foo'");
    }

    #[test]
    fn test_shell_quote_embedded_quote() {
        assert_eq!(shell_quote(&PathBuf::from("/tmp/it's")), "'/tmp/it'\\''s'");
    }

    #[test]
    fn test_partition_world_writable_is_safe() {
        let f = make_finding(
            Category::PermissionIssue,
            Severity::High,
            "World-writable file or directory",
            Target::Path(PathBuf::from("/tmp/ww")),
        );
        let findings = [f];
        let (safe, review) = FixScriptGenerator::partition(&findings);
        assert_eq!(safe.len(), 1);
        assert_eq!(review.len(), 0);
    }

    #[test]
    fn test_partition_suid_is_review() {
        let f = make_finding(
            Category::PermissionIssue,
            Severity::High,
            "SUID binary detected",
            Target::Path(PathBuf::from("/usr/bin/sudo")),
        );
        let findings = [f];
        let (safe, review) = FixScriptGenerator::partition(&findings);
        assert_eq!(safe.len(), 0);
        assert_eq!(review.len(), 1);
    }

    #[test]
    fn test_partition_broken_symlink_is_review() {
        let f = make_finding(
            Category::BrokenSymlink,
            Severity::Medium,
            "Broken symlink detected",
            Target::Path(PathBuf::from("/opt/homebrew/bin/foo")),
        );
        let findings = [f];
        let (safe, review) = FixScriptGenerator::partition(&findings);
        assert_eq!(safe.len(), 0);
        assert_eq!(review.len(), 1);
    }

    #[test]
    fn test_partition_discovery_categories_are_dropped() {
        let f = make_finding(
            Category::PythonEnv,
            Severity::Info,
            "Python env",
            Target::Path(PathBuf::from("/tmp/venv")),
        );
        let findings = [f];
        let (safe, review) = FixScriptGenerator::partition(&findings);
        assert_eq!(safe.len(), 0);
        assert_eq!(review.len(), 0);
    }

    #[test]
    fn test_build_script_has_confirmation_gate() {
        let f = make_finding(
            Category::Cache,
            Severity::Low,
            "Cache file detected",
            Target::Path(PathBuf::from("/tmp/cache")),
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("--yes"));
        assert!(script.contains("# rm -rf"));
    }

    #[test]
    fn test_build_script_world_writable_is_active() {
        let f = make_finding(
            Category::PermissionIssue,
            Severity::High,
            "World-writable file or directory",
            Target::Path(PathBuf::from("/tmp/ww")),
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("chmod o-w '/tmp/ww'"));
        // Should NOT be commented out.
        assert!(!script.contains("# chmod o-w '/tmp/ww'"));
    }

    #[test]
    fn test_build_script_destructive_is_commented() {
        let f = make_finding(
            Category::Cache,
            Severity::Low,
            "Cache file detected",
            Target::Path(PathBuf::from("/tmp/cache")),
        );
        let script = FixScriptGenerator::build_script(&[f]);
        assert!(script.contains("# rm -rf -- '/tmp/cache'"));
    }

    #[test]
    fn test_write_creates_executable_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fixes.sh");
        let gen = FixScriptGenerator::new(path.clone());
        let f = make_finding(
            Category::Cache,
            Severity::Low,
            "Cache file detected",
            Target::Path(PathBuf::from("/tmp/cache")),
        );
        let written = gen.write(&[f]).unwrap();
        assert_eq!(written, path);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("#!/bin/bash"));
        assert!(content.contains("# rm -rf"));
    }
}
