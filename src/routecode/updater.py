import json
import os
import platform
import subprocess
import sys
import tempfile
import urllib.request
from dataclasses import dataclass
from typing import Optional

GITHUB_REPO = "anasx07/routecode"
GITHUB_API_LATEST = f"https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
GITHUB_RELEASES_PAGE = f"https://github.com/{GITHUB_REPO}/releases"


@dataclass
class UpdateInfo:
    current_version: str
    latest_version: str
    is_available: bool = False
    download_url: Optional[str] = None
    release_notes: str = ""
    release_url: str = ""
    install_type: str = "unknown"
    error: Optional[str] = None

    def __bool__(self):
        return self.is_available


def _get_current_version() -> str:
    from . import __version__

    return __version__


def _get_install_type() -> str:
    if getattr(sys, "frozen", False):
        return "binary"
    return "pip"


def _compare_versions(a: str, b: str) -> int:
    """Return 1 if a > b, -1 if a < b, 0 if equal. Handles dev/pre-release segments."""
    a_parts = a.split(".")
    b_parts = b.split(".")

    for i in range(max(len(a_parts), len(b_parts))):
        try:
            av = int(
                a_parts[i].split("+")[0].split("dev")[0] if i < len(a_parts) else 0
            )
        except (ValueError, IndexError):
            av = 0
        try:
            bv = int(
                b_parts[i].split("+")[0].split("dev")[0] if i < len(b_parts) else 0
            )
        except (ValueError, IndexError):
            bv = 0
        if av > bv:
            return 1
        elif av < bv:
            return -1
    return 0


def _get_platform_asset_name() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "windows":
        return "routecode-cli-windows-x86_64.exe"
    elif system == "darwin":
        return (
            "routecode-cli-macos-arm64"
            if machine in ("arm64", "aarch64")
            else "routecode-cli-macos-x86_64"
        )
    elif system == "linux":
        return "routecode-cli-linux-x86_64"

    return "routecode-cli-linux-x86_64"


def check_for_update() -> UpdateInfo:
    """Check GitHub for a newer release. Non-blocking, 5s timeout."""
    current = _get_current_version()
    install_type = _get_install_type()

    try:
        req = urllib.request.Request(GITHUB_API_LATEST)
        req.add_header("Accept", "application/vnd.github.v3+json")
        req.add_header("User-Agent", "routecode-updater")

        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read().decode())

        latest_tag = str(data.get("tag_name", ""))
        latest_version = latest_tag.lstrip("v").lstrip("V")

        if not latest_version:
            return UpdateInfo(
                current_version=current,
                latest_version=current,
                install_type=install_type,
                error="Could not parse latest version from GitHub",
            )

        is_newer = _compare_versions(latest_version, current) > 0

        download_url = None
        if is_newer and install_type == "binary":
            platform_asset = _get_platform_asset_name()
            for asset in data.get("assets", []):
                if asset.get("name") == platform_asset:
                    download_url = asset.get("browser_download_url")
                    break

        return UpdateInfo(
            current_version=current,
            latest_version=latest_version,
            is_available=is_newer,
            download_url=download_url,
            release_notes=data.get("body", ""),
            release_url=data.get("html_url", GITHUB_RELEASES_PAGE),
            install_type=install_type,
        )
    except urllib.error.HTTPError as e:
        return UpdateInfo(
            current_version=current,
            latest_version=current,
            install_type=install_type,
            error=f"GitHub API returned {e.code}",
        )
    except Exception as e:
        return UpdateInfo(
            current_version=current,
            latest_version=current,
            install_type=install_type,
            error=f"Update check failed: {e}",
        )


def perform_update(update_info: UpdateInfo, console=None) -> bool:
    """Download and install the update. Returns True on success."""
    if not update_info.is_available:
        if console:
            console.print("[dim]Already up to date.[/dim]")
        return False

    if update_info.install_type == "pip":
        return _pip_update(update_info, console)

    if not update_info.download_url:
        if console:
            console.print(
                f"[warning]No binary download found for your platform.[/warning]\n"
                f"[dim]Open {update_info.release_url} to download manually.[/dim]"
            )
        return False

    return _binary_update(update_info, console)


def _pip_update(update_info: UpdateInfo, console=None) -> bool:
    if console:
        console.print(
            f"[accent]Updating[/accent] from {update_info.current_version} "
            f"→ [white]{update_info.latest_version}[/white] via pip..."
        )
    try:
        subprocess.check_call(
            [sys.executable, "-m", "pip", "install", "--upgrade", "routecode"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if console:
            console.print(
                "[success]Update installed. Restart RouteCode to use the new version.[/success]"
            )
        return True
    except subprocess.CalledProcessError as e:
        if console:
            console.print(f"[error]pip install failed: {e}[/error]")
        return False


def _binary_update(update_info: UpdateInfo, console=None) -> bool:
    tmp_path = None
    try:
        tmp_fd, tmp_path = tempfile.mkstemp(prefix="routecode_update_")
        os.close(tmp_fd)

        if console:
            console.print(
                f"[accent]Downloading[/accent] {update_info.latest_version}...",
                end="\r",
            )

        def _download_progress(block_count, block_size, total_size):
            if not console:
                return
            if total_size > 0:
                downloaded = block_count * block_size
                pct = min(downloaded / total_size * 100, 100)
                console.print(
                    f"[accent]Downloading[/accent] {update_info.latest_version} "
                    f"({_format_size(downloaded)} / {_format_size(total_size)}, {pct:.0f}%)...",
                    end="\r",
                )
            else:
                downloaded = block_count * block_size
                console.print(
                    f"[accent]Downloading[/accent] {update_info.latest_version} "
                    f"({_format_size(downloaded)})...",
                    end="\r",
                )

        urllib.request.urlretrieve(
            update_info.download_url, tmp_path, _download_progress
        )

        if console:
            console.print(
                f"[accent]Downloading[/accent] {update_info.latest_version} "
                f"([success]done[/success])   "
            )

        current_exe = sys.executable
        _platform = platform.system()

        # Define persistent pending update path
        update_dir = os.path.dirname(current_exe)
        if not os.access(update_dir, os.W_OK):
            # Fallback to home dir if we can't write to exe dir
            update_dir = os.path.expanduser("~/.routecode")
            os.makedirs(update_dir, exist_ok=True)

        pending_ext = ".exe" if _platform == "Windows" else ""
        pending_path = os.path.join(update_dir, f"update_pending{pending_ext}")

        # Move temp file to persistent pending path
        import shutil

        shutil.move(tmp_path, pending_path)

        if console:
            console.print(
                f"\n[success]✔[/success] Update {update_info.latest_version} downloaded.\n"
                "[info]The update will be installed automatically the next time you start RouteCode.[/info]"
            )
        return True

    except Exception as e:
        if console:
            console.print(f"[error]Update failed: {e}[/error]")
        if tmp_path:
            try:
                os.unlink(tmp_path)
            except Exception:
                pass
        return False


def apply_pending_update():
    """Check for and apply any pending updates at startup."""
    if not getattr(sys, "frozen", False):
        return

    current_exe = sys.executable
    _platform = platform.system()
    update_dir = os.path.dirname(current_exe)
    pending_ext = ".exe" if _platform == "Windows" else ""
    pending_path = os.path.join(update_dir, f"update_pending{pending_ext}")

    # Check fallback dir too
    if not os.path.exists(pending_path):
        pending_path = os.path.expanduser(f"~/.routecode/update_pending{pending_ext}")

    if not os.path.exists(pending_path):
        return

    # Trigger the swap and exit
    if _platform == "Windows":
        _replace_and_relaunch_windows(pending_path, current_exe)
    else:
        _replace_and_relaunch_unix(pending_path, current_exe)
    sys.exit(0)


def _replace_and_relaunch_windows(new_exe: str, current_exe: str, console=None) -> bool:
    script_fd, script_path = tempfile.mkstemp(
        suffix=".bat", prefix="routecode_install_"
    )
    os.close(script_fd)

    with open(script_path, "w") as f:
        f.write(
            "@echo off\n"
            "echo RouteCode CLI Updater\n"
            f":retry\n"
            f"ping -n 2 127.0.0.1 >nul\n"
            f"echo Installing...\n"
            f'move /Y "{new_exe}" "{current_exe}"\n'
            f'if exist "{new_exe}" goto retry\n'
            f"echo Done. Starting RouteCode...\n"
            f'start "" "{current_exe}"\n'
            f'del "%~f0"\n'
        )

    subprocess.Popen(
        f'cmd /c "{script_path}"',
        shell=True,
        creationflags=subprocess.CREATE_NO_WINDOW
        if hasattr(subprocess, "CREATE_NO_WINDOW")
        else 0,
    )
    return True


def _replace_and_relaunch_unix(new_exe: str, current_exe: str, console=None) -> bool:
    script_fd, script_path = tempfile.mkstemp(suffix=".sh", prefix="routecode_install_")
    os.close(script_fd)

    os.chmod(new_exe, 0o755)

    with open(script_path, "w") as f:
        f.write(
            "#!/bin/bash\n"
            "sleep 1\n"
            f'mv -f "{new_exe}" "{current_exe}"\n'
            f'chmod +x "{current_exe}"\n'
            f'echo "Update installed. Starting RouteCode..."\n'
            f'exec "{current_exe}"\n'
            f'rm -f "$0"\n'
        )

    os.chmod(script_path, 0o755)

    subprocess.Popen(
        [script_path],
        start_new_session=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        close_fds=True,
    )
    return True


def _format_size(size: int) -> str:
    if size < 1024:
        return f"{size} B"
    elif size < 1024 * 1024:
        return f"{size / 1024:.1f} KB"
    else:
        return f"{size / (1024 * 1024):.1f} MB"
