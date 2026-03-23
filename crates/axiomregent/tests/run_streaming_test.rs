// tests/run_streaming_test.rs

use axiomregent::run_tools::RunTools;
use std::thread;
use std::time::Duration;

#[test]
fn test_run_lifecycle_and_streaming() {
    let dir = tempfile::tempdir().unwrap();
    let tools = RunTools::new(dir.path());

    // 1. Execute a non-existent skill
    let result = tools
        .execute("non-existent-skill".to_string(), None)
        .unwrap();
    let run_id = result["run_id"].as_str().unwrap().to_string();

    // 2. Wait for it to fail (polls status until terminal state)
    let mut status_json = serde_json::Value::Null;
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(100));
        status_json = tools.status(&run_id).unwrap();
        let status_str = status_json["status"].as_str().unwrap();
        if status_str == "fail" || status_str == "pass" {
            break;
        }
    }

    assert_eq!(status_json["status"], "fail");
    assert_eq!(status_json["run_id"], run_id.as_str());
    assert!(status_json["start_time"].is_string());
    assert!(status_json["end_time"].is_string());
    assert!(status_json["exit_code"].is_number());

    // 3. Seed the log file with known multi-line content for pagination tests
    let logs_path = dir
        .path()
        .join(".axiomregent/run/logs")
        .join(format!("{}.log", run_id));
    let known_lines = "line0\nline1\nline2\nline3\nline4\n";
    std::fs::write(&logs_path, known_lines).unwrap();

    // 4. Full log fetch
    let all = tools.logs(&run_id, None, None).unwrap();
    assert_eq!(all["total"], 5);
    assert_eq!(all["truncated"], false);
    assert_eq!(all["lines"].as_array().unwrap().len(), 5);

    // 5. Limit — first 3 lines
    let partial = tools.logs(&run_id, None, Some(3)).unwrap();
    let lines = partial["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line0");
    assert_eq!(lines[2], "line2");
    assert_eq!(partial["truncated"], true);
    assert_eq!(partial["total"], 5);

    // 6. Offset — skip first 2 lines
    let offset = tools.logs(&run_id, Some(2), None).unwrap();
    let lines = offset["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line2");
    assert_eq!(lines[2], "line4");
    assert_eq!(offset["truncated"], false);

    // 7. Offset + limit
    let slice = tools.logs(&run_id, Some(1), Some(2)).unwrap();
    let lines = slice["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(slice["truncated"], true);
}
