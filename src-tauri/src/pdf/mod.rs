//! PDF generation utilities (Slice 11b).
//!
//! This module currently exposes a single entry point: [`report_pdf::render_report`],
//! which renders a `ReportData` payload to an A4 landscape PDF on disk. The
//! layout is intentionally fixed (no runtime configurability) so the table
//! stays readable and predictable for operational reports.
//!
//! Future slices can extend this module with portrait layouts, additional
//! summary pages, or template-based variants without disturbing the existing
//! `render_report` contract.

pub mod report_pdf;
