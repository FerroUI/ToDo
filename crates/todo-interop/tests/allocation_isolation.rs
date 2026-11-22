use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use todo_domain::{AppState, Date};
use todo_interop::{allocation_budget::ParserAllocator, import_pst_owned};

#[global_allocator]
static ALLOCATOR: ParserAllocator = ParserAllocator::new();

#[test]
fn parser_budget_contains_real_pst_allocations() {
    if let Ok(case) = std::env::var("TODO_PST_BUDGET_TEST_CASE") {
        // Enabled only in this disposable child, before input allocation. The
        // malformed file contains a structurally valid 2^27 category count.
        ALLOCATOR.enable(16 * 1024 * 1024).unwrap();
        let bytes = if case == "valid" {
            include_bytes!("fixtures/tasks.pst").as_slice()
        } else {
            include_bytes!("fixtures/malformed-count.pst").as_slice()
        };
        let result = import_pst_owned(
            bytes.to_vec(),
            &AppState::new(),
            Date::from_ymd_opt(2026, 11, 22).unwrap(),
        );
        if case == "valid" {
            assert_eq!(result.unwrap().archive.document.tasks.len(), 2);
        } else {
            // A future decoder may reject the count before allocating; that is
            // also safe. Today's upstream reader aborts this child on OOM.
            assert!(result.is_err());
        }
        return;
    }
    for case in ["valid", "malformed"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parser_budget_contains_real_pst_allocations",
                "--nocapture",
            ])
            .env("TODO_PST_BUDGET_TEST_CASE", case)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("The isolated {case} PST parser exceeded its deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() || (case == "malformed" && error.contains("memory allocation")),
            "Unexpected isolated {case} parser failure: {error}"
        );
    }
}
