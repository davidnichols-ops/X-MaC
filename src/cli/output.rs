use anyhow::Result;
use std::io::Write;

use crate::cli::args::{GlobalArgs, OutputFormat};
use crate::core::types::Finding;

pub struct OutputWriter {
    format: OutputFormat,
    writer: Box<dyn Write + Send>,
    buffer: Vec<Finding>,
    /// When true, findings are buffered in addition to being streamed so the
    /// fix-script generator can consume them after the scan.
    buffer_for_fix_script: bool,
}

impl OutputWriter {
    pub fn new(args: &GlobalArgs) -> Self {
        let writer: Box<dyn Write + Send> = match &args.output {
            Some(path) => match std::fs::File::create(path) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    eprintln!(
                        "Failed to create output file {}: {}. Falling back to stdout.",
                        path.display(),
                        e
                    );
                    Box::new(std::io::stdout())
                }
            },
            None => Box::new(std::io::stdout()),
        };

        let buffer_for_fix_script = args.fix_script.is_some();

        Self {
            format: args.format,
            writer,
            buffer: Vec::new(),
            buffer_for_fix_script,
        }
    }

    pub fn write_finding(&mut self, finding: &Finding) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                serde_json::to_writer(&mut self.writer, finding)?;
                writeln!(self.writer)?;
                if self.buffer_for_fix_script {
                    self.buffer.push(finding.clone());
                }
            }
            OutputFormat::JsonPretty | OutputFormat::Report | OutputFormat::Html => {
                self.buffer.push(finding.clone());
            }
            OutputFormat::Csv => {
                // CSV: one finding per line, write header on first finding
                if self.buffer.is_empty() {
                    writeln!(
                        self.writer,
                        "id,engine,severity,category,title,description,size_bytes,target"
                    )?;
                }
                self.buffer.push(finding.clone());
                let target_str = match &finding.target {
                    crate::core::types::Target::Path(p) => p.display().to_string(),
                    crate::core::types::Target::Process(pid) => format!("pid:{}", pid),
                    crate::core::types::Target::Port(p) => format!("port:{}", p),
                    crate::core::types::Target::EnvironmentVariable(v) => v.clone(),
                    crate::core::types::Target::LaunchdLabel(l) => l.clone(),
                    crate::core::types::Target::Package(p) => p.clone(),
                };
                // Escape quotes in CSV fields
                let esc = |s: &str| s.replace('"', "\"\"");
                let engine_str = format!("{:?}", finding.engine);
                let severity_str = format!("{:?}", finding.severity);
                let category_str = format!("{:?}", finding.category);
                writeln!(
                    self.writer,
                    "{},{},{},{},\"{}\",\"{}\",{},\"{}\"",
                    finding.id,
                    engine_str,
                    severity_str,
                    category_str,
                    esc(&finding.title),
                    esc(&finding.description),
                    finding.size_bytes.unwrap_or(0),
                    esc(&target_str),
                )?;
            }
        }
        Ok(())
    }

    pub fn take_findings(&mut self) -> Vec<Finding> {
        std::mem::take(&mut self.buffer)
    }

    pub fn write_report(&mut self, report: &crate::core::types::ScanReport) -> Result<()> {
        serde_json::to_writer_pretty(&mut self.writer, report)?;
        writeln!(self.writer)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        match self.format {
            OutputFormat::JsonPretty if !self.buffer.is_empty() => {
                serde_json::to_writer_pretty(&mut self.writer, &self.buffer)?;
                writeln!(self.writer)?;
            }
            OutputFormat::Html if !self.buffer.is_empty() => {
                self.write_html_report()?;
            }
            _ => {}
        }
        self.writer.flush()?;
        Ok(())
    }

    fn write_html_report(&mut self) -> Result<()> {
        let total_reclaimable: u64 = self.buffer.iter().filter_map(|f| f.size_bytes).sum();

        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"utf-8\">\n");
        html.push_str("<title>X-MaC Scan Report</title>\n");
        html.push_str("<style>\n");
        html.push_str(include_str!("report.css"));
        html.push_str("</style>\n</head>\n<body>\n");
        html.push_str("<h1>X-MaC Scan Report</h1>\n");
        html.push_str(&format!(
            "<p><strong>Findings:</strong> {}</p>\n",
            self.buffer.len()
        ));
        html.push_str(&format!(
            "<p><strong>Reclaimable:</strong> {}</p>\n",
            crate::util::disk::format_bytes(total_reclaimable)
        ));
        html.push_str("<table>\n<thead>\n<tr>\n");
        html.push_str("<th>Severity</th><th>Category</th><th>Engine</th><th>Title</th><th>Description</th><th>Size</th><th>Target</th>\n");
        html.push_str("</tr>\n</thead>\n<tbody>\n");

        for finding in &self.buffer {
            let target = match &finding.target {
                crate::core::types::Target::Path(p) => p.display().to_string(),
                crate::core::types::Target::Process(pid) => format!("pid:{}", pid),
                crate::core::types::Target::Port(p) => format!("port:{}", p),
                crate::core::types::Target::EnvironmentVariable(v) => v.clone(),
                crate::core::types::Target::LaunchdLabel(l) => l.clone(),
                crate::core::types::Target::Package(p) => p.clone(),
            };
            html.push_str("<tr>\n");
            html.push_str(&format!(
                "<td class=\"{}\">{}</td>\n",
                html_escape(&format!("{:?}", finding.severity).to_lowercase()),
                html_escape(&format!("{:?}", finding.severity))
            ));
            html.push_str(&format!(
                "<td>{}</td>\n",
                html_escape(&format!("{:?}", finding.category))
            ));
            html.push_str(&format!(
                "<td>{}</td>\n",
                html_escape(&format!("{:?}", finding.engine))
            ));
            html.push_str(&format!("<td>{}</td>\n", html_escape(&finding.title)));
            html.push_str(&format!("<td>{}</td>\n", html_escape(&finding.description)));
            html.push_str(&format!(
                "<td>{}</td>\n",
                finding
                    .size_bytes
                    .map(crate::util::disk::format_bytes)
                    .unwrap_or_default()
            ));
            html.push_str(&format!("<td>{}</td>\n", html_escape(&target)));
            html.push_str("</tr>\n");
        }

        html.push_str("</tbody>\n</table>\n");
        html.push_str("</body>\n</html>\n");

        self.writer.write_all(html.as_bytes())?;
        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
