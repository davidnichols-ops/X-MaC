use anyhow::Result;
use std::io::Write;

use crate::cli::args::{GlobalArgs, OutputFormat};
use crate::core::types::Finding;

pub struct OutputWriter {
    format: OutputFormat,
    writer: Box<dyn Write + Send>,
    buffer: Vec<Finding>,
    is_pretty: bool,
}

impl OutputWriter {
    pub fn new(args: &GlobalArgs) -> Self {
        let writer: Box<dyn Write + Send> = match &args.output {
            Some(path) => Box::new(std::fs::File::create(path).expect("Failed to create output file")),
            None => Box::new(std::io::stdout()),
        };

        let is_pretty = matches!(args.format, OutputFormat::JsonPretty);

        Self {
            format: args.format,
            writer,
            buffer: Vec::new(),
            is_pretty,
        }
    }

    pub fn write_finding(&mut self, finding: &Finding) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                serde_json::to_writer(&mut self.writer, finding)?;
                writeln!(self.writer)?;
            }
            OutputFormat::JsonPretty | OutputFormat::Report => {
                self.buffer.push(finding.clone());
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
        if self.is_pretty && !self.buffer.is_empty() {
            serde_json::to_writer_pretty(&mut self.writer, &self.buffer)?;
            writeln!(self.writer)?;
        }
        self.writer.flush()?;
        Ok(())
    }
}
