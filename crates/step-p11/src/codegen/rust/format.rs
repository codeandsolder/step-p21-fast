use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Format generated Rust source using the `rustfmt` executable.
///
/// # Errors
/// Returns the underlying I/O error when the formatter cannot be started,
/// cannot accept the source, or produces invalid UTF-8. Non-success exit
/// status includes the formatter's stderr in the returned error.
pub fn rustfmt(source: String) -> io::Result<String> {
    let mut child = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Feed stdin from another thread so large generated Rust modules do not
    // deadlock when the formatter fills stdout before consuming all input.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("rustfmt stdin pipe not available"))?;
    let writer = std::thread::spawn(move || stdin.write_all(source.as_bytes()));

    let output = child.wait_with_output()?;
    writer
        .join()
        .map_err(|_| io::Error::other("rustfmt input writer panicked"))??;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "rustfmt exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    String::from_utf8(output.stdout).map_err(io::Error::other)
}
