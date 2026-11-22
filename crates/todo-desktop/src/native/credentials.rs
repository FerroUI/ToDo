use std::io::{Read, Write};
use std::time::Duration;

/// A blocked or prompting credential provider must not outlive its caller.
/// Input, output, and the timeout contain no secret-bearing diagnostics.
pub fn child_main() -> i32 {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(6));
        std::process::exit(3);
    });
    #[cfg(target_os = "linux")]
    {
        let parent = unsafe { libc::getppid() };
        if parent <= 1
            || unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0
            || unsafe { libc::getppid() } != parent
        {
            return 3;
        }
    }
    let mut request = Vec::new();
    if std::io::stdin()
        .take(todo_credentials::MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut request)
        .is_err()
    {
        return 2;
    }
    let mut response = todo_credentials::handle_request(&request);
    request.fill(0);
    let mut output = std::io::stdout().lock();
    let result = output.write_all(&response).and_then(|_| output.flush());
    response.fill(0);
    if result.is_ok() {
        0
    } else {
        2
    }
}
