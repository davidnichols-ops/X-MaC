//! Plist parsing for LaunchAgent/LaunchDaemon property lists.
//!
//! Implements operation 150 (parse plist files) using the `plist` crate.
//! Extracts label, program arguments, start interval, run-at-load, and
//! other fields relevant to startup impact analysis.

use std::path::Path;
use std::time::Duration;

/// A parsed LaunchAgent or LaunchDaemon plist entry.
#[derive(Debug, Clone, Default)]
pub struct ParsedPlist {
    /// The `Label` key — unique identifier for the service.
    pub label: String,
    /// The `Program` key — path to the executable (if present).
    pub program: Option<String>,
    /// The `ProgramArguments` key — command and arguments.
    pub program_arguments: Vec<String>,
    /// The `RunAtLoad` key — whether the service starts at boot/login.
    pub run_at_load: bool,
    /// The `KeepAlive` key — whether the service is kept alive (auto-restart).
    pub keep_alive: bool,
    /// The `StartInterval` key — seconds between scheduled runs (if any).
    pub start_interval: Option<u64>,
    /// The `StartCalendarInterval` key — cron-like schedule (if any).
    pub start_calendar_interval: bool,
    /// The `Disabled` key — whether the service is disabled.
    pub disabled: bool,
    /// The `ProcessType` key — e.g. "Background", "Interactive", "Adaptive".
    pub process_type: Option<String>,
}

impl ParsedPlist {
    /// Whether this service launches at boot/login.
    #[allow(dead_code)]
    pub fn launches_at_boot(&self) -> bool {
        self.run_at_load || self.keep_alive
    }

    /// Whether this is a scheduled (delayed/periodic) service.
    #[allow(dead_code)]
    pub fn is_scheduled(&self) -> bool {
        self.start_interval.is_some() || self.start_calendar_interval
    }

    /// The effective command string (program + arguments or just arguments).
    pub fn command_string(&self) -> String {
        if !self.program_arguments.is_empty() {
            self.program_arguments.join(" ")
        } else if let Some(ref prog) = self.program {
            prog.clone()
        } else {
            String::new()
        }
    }

    /// The startup delay implied by StartInterval, as a Duration.
    #[allow(dead_code)]
    pub fn start_delay(&self) -> Option<Duration> {
        self.start_interval.map(Duration::from_secs)
    }
}

/// Parse a plist file at the given path. (op 150)
///
/// Returns `Ok(ParsedPlist)` on success, or an error if the file cannot
/// be read or is not a valid plist.
pub fn parse_plist(path: &Path) -> Result<ParsedPlist, plist::Error> {
    let value = plist::Value::from_file(path)?;
    Ok(plist_value_to_parsed(&value))
}

/// Parse a plist from an in-memory bytes buffer (for testing).
#[allow(dead_code)]
pub fn parse_plist_bytes(data: &[u8]) -> Result<ParsedPlist, plist::Error> {
    let cursor = std::io::Cursor::new(data);
    let value = plist::Value::from_reader(cursor)?;
    Ok(plist_value_to_parsed(&value))
}

fn plist_value_to_parsed(value: &plist::Value) -> ParsedPlist {
    let dict = match value.as_dictionary() {
        Some(d) => d,
        None => return ParsedPlist::default(),
    };

    let label = dict
        .get("Label")
        .and_then(|v| v.as_string())
        .unwrap_or("")
        .to_string();

    let program = dict
        .get("Program")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let program_arguments = dict
        .get("ProgramArguments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_string().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let run_at_load = dict
        .get("RunAtLoad")
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);

    let keep_alive = dict
        .get("KeepAlive")
        .map(|v| match v {
            plist::Value::Boolean(b) => *b,
            plist::Value::Dictionary(d) => !d.is_empty(),
            _ => false,
        })
        .unwrap_or(false);

    let start_interval = dict
        .get("StartInterval")
        .and_then(|val| val.as_unsigned_integer())
        .or_else(|| {
            dict.get("StartInterval")
                .and_then(|val| val.as_signed_integer())
                .filter(|i| *i >= 0)
                .map(|i| i as u64)
        });

    let start_calendar_interval = dict.get("StartCalendarInterval").is_some();

    let disabled = dict
        .get("Disabled")
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);

    let process_type = dict
        .get("ProcessType")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    ParsedPlist {
        label,
        program,
        program_arguments,
        run_at_load,
        keep_alive,
        start_interval,
        start_calendar_interval,
        disabled,
        process_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_test_plist(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_xml_plist_basic() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/example</string>
        <string>--flag</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "com.example.agent.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert_eq!(parsed.label, "com.example.agent");
        assert_eq!(parsed.program_arguments, vec!["/usr/bin/example", "--flag"]);
        assert!(parsed.run_at_load);
        assert!(parsed.keep_alive);
        assert!(parsed.launches_at_boot());
        assert!(!parsed.is_scheduled());
    }

    #[test]
    fn test_parse_plist_with_start_interval() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.scheduled</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/scheduled-task</string>
    </array>
    <key>StartInterval</key>
    <integer>300</integer>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "com.example.scheduled.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert_eq!(parsed.label, "com.example.scheduled");
        assert_eq!(parsed.start_interval, Some(300));
        assert!(parsed.is_scheduled());
        assert_eq!(parsed.start_delay(), Some(Duration::from_secs(300)));
        assert!(!parsed.launches_at_boot());
    }

    #[test]
    fn test_parse_plist_with_process_type() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.background</string>
    <key>Program</key>
    <string>/usr/bin/background-helper</string>
    <key>ProcessType</key>
    <string>Background</string>
    <key>Disabled</key>
    <true/>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "com.example.background.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert_eq!(parsed.label, "com.example.background");
        assert_eq!(
            parsed.program,
            Some("/usr/bin/background-helper".to_string())
        );
        assert_eq!(parsed.process_type.as_deref(), Some("Background"));
        assert!(parsed.disabled);
        assert_eq!(parsed.command_string(), "/usr/bin/background-helper");
    }

    #[test]
    fn test_parse_plist_keep_alive_dict() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.alive</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/daemon</string>
    </array>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "com.example.alive.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert!(parsed.keep_alive);
        assert!(parsed.launches_at_boot());
    }

    #[test]
    fn test_parse_plist_calendar_interval() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.cron</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/cron-job</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>30</integer>
    </dict>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "com.example.cron.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert!(parsed.start_calendar_interval);
        assert!(parsed.is_scheduled());
    }

    #[test]
    fn test_parse_plist_empty_dict() {
        let tmp = TempDir::new().unwrap();
        let plist_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
</dict>
</plist>"#;
        let path = write_test_plist(tmp.path(), "empty.plist", plist_xml);
        let parsed = parse_plist(&path).unwrap();

        assert_eq!(parsed.label, "");
        assert!(parsed.program_arguments.is_empty());
        assert!(!parsed.run_at_load);
    }

    #[test]
    fn test_parse_plist_invalid_file() {
        let tmp = TempDir::new().unwrap();
        let path = write_test_plist(tmp.path(), "invalid.plist", "not a plist");
        assert!(parse_plist(&path).is_err());
    }

    #[test]
    fn test_command_string_uses_program_arguments() {
        let parsed = ParsedPlist {
            label: "test".to_string(),
            program: Some("/usr/bin/foo".to_string()),
            program_arguments: vec!["/usr/bin/bar".to_string(), "--arg".to_string()],
            ..Default::default()
        };
        assert_eq!(parsed.command_string(), "/usr/bin/bar --arg");
    }

    #[test]
    fn test_command_string_falls_back_to_program() {
        let parsed = ParsedPlist {
            label: "test".to_string(),
            program: Some("/usr/bin/foo".to_string()),
            program_arguments: vec![],
            ..Default::default()
        };
        assert_eq!(parsed.command_string(), "/usr/bin/foo");
    }
}
