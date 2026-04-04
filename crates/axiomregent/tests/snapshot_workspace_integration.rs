// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use anyhow::Result;
use std::process::Command;

#[test]
fn test_agent_tools_exist() -> Result<()> {
    // This test verifies that the binary exposes the agent tools.
    // It runs the binary with `tools/list` request.

    let bin = env!("CARGO_BIN_EXE_axiomregent");

    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.as_mut().unwrap();
    use std::io::Write;

    // Send initialize
    let init_req = r#"{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}},"id":1}"#;
    write!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        init_req.len(),
        init_req
    )?;

    // Read response (skip for now, assume success)

    // Send tools/list
    let list_req = r#"{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}"#;
    write!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        list_req.len(),
        list_req
    )?;

    // Read output
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if tools are present in output
    assert!(
        stdout.contains("workspace.write_file"),
        "Missing workspace.write_file in tools/list"
    );
    assert!(
        stdout.contains("checkpoint.create") || stdout.contains("workspace.apply_patch"),
        "Missing expected tools in tools/list"
    );

    Ok(())
}
