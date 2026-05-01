// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Factory pipeline stages that execute as in-process Rust code rather than
//! through the LLM-agent dispatch path. Today this is just `s-1-extract`
//! (spec 120); future Rust stages live alongside.

pub mod s_minus_1_extract;
