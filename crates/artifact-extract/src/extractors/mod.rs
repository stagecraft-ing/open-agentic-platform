// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Per-format body extractors. Each `extract(path) -> Result<String>`
//! produces the body portion of the resulting `.txt` (the header is added
//! by the caller in `lib.rs`).

pub mod docx;
pub mod json;
pub mod pbix;
pub mod pdf;
pub mod pptx;
pub mod xlsx;
pub mod zip;

pub(crate) mod ooxml_text;
