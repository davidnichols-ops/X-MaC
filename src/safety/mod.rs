// Safety — rule-driven cleanup classification with three-tier trust model
//
// Inspired by Gargantua's safety architecture: every file targeted for
// cleanup traces to a named rule with a rating (safe/review/protected).
// No file is removable without a rule. Protected paths are hard-blocked
// regardless of what any model or user says.
//
// Rule format (YAML):
//   name: xcode_derived_data
//   description: "Xcode build intermediates — regenerated on next compile"
//   rating: safe
//   paths:
//     - "~/Library/Developer/Xcode/DerivedData/**"
//   confidence: 98
//   upstream_commit: "a7c19e4"
//
//   name: slack_cache
//   description: "Slack cached workspace data — re-downloads on launch"
//   rating: review
//   paths:
//     - "~/Library/Application Support/Slack/Cache/**"
//   confidence: 71
//
//   name: keychain
//   description: "Credential store — never removable"
//   rating: protected
//   paths:
//     - "~/Library/Keychains/**"
//   confidence: 100

#![allow(dead_code, unused_imports)]

pub mod rule_engine;
pub mod rules;
