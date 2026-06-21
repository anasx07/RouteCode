use crate::core::ToolResult;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

pub async fn run(command_str: &str) -> Result<Output, std::io::Error> {
    let output = if cfg!(target_os = "windows") {
        TokioCommand::new("cmd")
            .args(["/C", command_str])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?
    } else {
        TokioCommand::new("sh")
            .args(["-c", command_str])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?
    };

    Ok(Output {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
        exit_code: output.status.code(),
    })
}

pub fn to_result(out: Output) -> ToolResult {
    if out.success {
        let mut result = out.stdout;
        if !out.stderr.is_empty() {
            result = format!("Stdout:\n{}\nStderr:\n{}", result, out.stderr);
        }
        ToolResult::success(result)
    } else {
        ToolResult::error(format!(
            "Command failed with exit code: {}\nStdout: {}\nStderr: {}",
            out.exit_code.unwrap_or(-1),
            out.stdout,
            out.stderr
        ))
    }
}
