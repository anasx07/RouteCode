def notify(title: str, message: str):
    try:
        import subprocess

        subprocess.run(
            [
                "powershell",
                "-Command",
                f"Add-Type -AssemblyName System.Windows.Forms; "
                f"[System.Windows.Forms.NotifyIcon]::new() | "
                f'% {{ $_.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon("powershell.exe"); '
                f'$_.BalloonTipIcon = "Info"; '
                f'$_.BalloonTipTitle = "{title}"; '
                f'$_.BalloonTipText = "{message}"; '
                f"$_.Visible = $true; "
                f"$_.ShowBalloonTip(3000) }}",
            ],
            timeout=5,
            capture_output=True,
        )
    except Exception:
        pass


def notify_task_complete(task_id: str, description: str):
    notify(f"Task {task_id}", f"{description[:60]} completed")
