#!/usr/bin/env python3
"""
release.py — Loom Release CLI
==============================
Run from the repo root:  python scripts/release.py
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import webbrowser
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

# ── Check deps available ───────────────────────────────────────────────────
try:
    from rich.console import Console
    from rich.panel import Panel
    from rich.table import Table
    from rich.text import Text
    from rich.prompt import Prompt, Confirm
    from rich.rule import Rule
    from rich.live import Live
    from rich.spinner import Spinner
    from rich.padding import Padding
    from rich import box
except ImportError:
    print("rich is not installed. Run: pip install rich")
    sys.exit(1)

# ──────────────────────────────────────────────────────────────────────────
REPO_ROOT = Path(__file__).resolve().parent.parent
ACCENT = "#ff4444"  # Loom lava red
DIM = "bright_black"
BOLD = "bold white"

console = Console(highlight=False)

# ═══════════════════════════════════════════════════════════════════════════
# Git helpers
# ═══════════════════════════════════════════════════════════════════════════


def git(*args: str, check=True, capture=True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        capture_output=capture,
        text=True,
        check=check,
    )


def git_out(*args: str) -> str:
    return git(*args).stdout.strip()


def find_tool(name: str) -> Optional[str]:
    """Find a tool, prioritising the local venv."""
    bin_dir = "Scripts" if os.name == "nt" else "bin"
    venv_tool = REPO_ROOT / "venv" / bin_dir / (f"{name}.exe" if os.name == "nt" else name)
    if venv_tool.exists():
        return str(venv_tool)
    return shutil.which(name)


def current_branch() -> str:
    return git_out("rev-parse", "--abbrev-ref", "HEAD")


def is_dirty() -> bool:
    return bool(git_out("status", "--porcelain"))


def latest_tag() -> Optional[str]:
    try:
        return git_out("describe", "--tags", "--abbrev=0")
    except subprocess.CalledProcessError:
        return None


def remote_url() -> str:
    try:
        url = git_out("remote", "get-url", "origin")
        # Normalise SSH → HTTPS
        url = re.sub(r"^git@github\.com:", "https://github.com/", url)
        return url.removesuffix(".git")
    except Exception:
        return ""


def commits_since(tag: Optional[str]) -> list[str]:
    ref = f"{tag}..HEAD" if tag else "HEAD"
    out = git_out("log", ref, "--oneline", "--no-decorate")
    return [line for line in out.splitlines() if line.strip()] if out else []


def has_upstream() -> bool:
    r = git("rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}", check=False)
    return r.returncode == 0


# ═══════════════════════════════════════════════════════════════════════════
# Version helpers
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class Version:
    major: int
    minor: int
    patch: int

    @classmethod
    def parse(cls, s: str) -> "Version":
        s = s.lstrip("v")
        parts = s.split(".")
        try:
            return cls(int(parts[0]), int(parts[1]), int(parts[2].split("-")[0]))
        except Exception:
            return cls(0, 0, 0)

    def bump(self, kind: str) -> "Version":
        if kind == "major":
            return Version(self.major + 1, 0, 0)
        if kind == "minor":
            return Version(self.major, self.minor + 1, 0)
        return Version(self.major, self.minor, self.patch + 1)

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"

    def tag(self) -> str:
        return f"v{self}"


# ═══════════════════════════════════════════════════════════════════════════
# UI helpers
# ═══════════════════════════════════════════════════════════════════════════


def header():
    console.print()
    title = Text()
    title.append("🪡  LOOM ", style=f"bold {ACCENT}")
    title.append("Release CLI", style=BOLD)
    console.print(Panel(title, border_style=ACCENT, padding=(0, 2)))
    console.print()


def step(n: int, total: int, msg: str):
    console.print(f"[{DIM}][{n}/{total}][/{DIM}] [bold]{msg}[/bold]")


def ok(msg: str):
    console.print(f"  [{ACCENT}]✔[/{ACCENT}] [white]{msg}[/white]")


def warn(msg: str):
    console.print(f"  [yellow]⚠[/yellow]  [yellow]{msg}[/yellow]")


def fail(msg: str):
    console.print(f"  [red]✘[/red]  [red]{msg}[/red]")
    sys.exit(1)


def run_step(label: str, *cmd: str) -> bool:
    """Run a shell command with a spinner. Returns True on success."""
    with Live(
        Spinner("dots", text=f"  [dim]{label}[/dim]"),
        console=console,
        refresh_per_second=12,
    ):
        r = subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True)
    if r.returncode == 0:
        ok(label)
        return True
    else:
        fail(f"{label} failed\n{r.stderr.strip() or r.stdout.strip()}")
        return False


# ═══════════════════════════════════════════════════════════════════════════
# Pre-flight checks
# ═══════════════════════════════════════════════════════════════════════════


def preflight(n_steps: int):
    step(1, n_steps, "Pre-flight checks")

    # Git available
    if not shutil.which("git"):
        fail("git not found on PATH.")
    ok("git found")

    # Inside a repo
    r = git("rev-parse", "--is-inside-work-tree", check=False)
    if r.returncode != 0:
        fail("Not inside a git repository.")
    ok("Inside git repository")

    # Remote configured
    url = remote_url()
    if not url:
        fail("No git remote 'origin' configured. Add one and retry.")
    ok(f"Remote: [cyan]{url}[/cyan]")

    # Branch check
    branch = current_branch()
    if branch != "main":
        warn(f"Current branch is [bold]{branch}[/bold], not [bold]main[/bold].")
        if not Confirm.ask(
            f"  Release from [bold]{branch}[/bold] anyway?", default=False
        ):
            console.print("[dim]  Aborted.[/dim]")
            sys.exit(0)
    else:
        ok(f"Branch: [bold]{branch}[/bold]")

    # Working tree clean
    if is_dirty():
        fail("Working tree has uncommitted changes. Commit or stash them first.")
    ok("Working tree is clean")

    # Upstream exists
    if not has_upstream():
        warn(
            "No upstream tracking branch set. Will push with [bold]--set-upstream origin[/bold]."
        )
    else:
        ok("Upstream tracking branch set")

    console.print()


# ═══════════════════════════════════════════════════════════════════════════
# Version selection
# ═══════════════════════════════════════════════════════════════════════════


def pick_version(n_steps: int) -> tuple[Version, str]:
    step(2, n_steps, "Choose version")

    tag = latest_tag()
    cur = Version.parse(tag) if tag else Version(0, 0, 0)
    label = f"v{cur}" if tag else "(no tags yet)"

    console.print(f"  Current version: [{ACCENT}]{label}[/{ACCENT}]")
    console.print()

    patch = cur.bump("patch")
    minor = cur.bump("minor")
    major = cur.bump("major")

    # Build a table of options
    t = Table(box=box.SIMPLE, show_header=False, padding=(0, 2))
    t.add_column("key", style=f"bold {ACCENT}", width=6)
    t.add_column("label", style="bold white", width=10)
    t.add_column("ver", style="cyan", width=12)
    t.add_column("desc", style=DIM)

    t.add_row("1", "patch", f"v{patch}", "Bug fixes, no new features")
    t.add_row("2", "minor", f"v{minor}", "New features, backwards compatible")
    t.add_row("3", "major", f"v{major}", "Breaking changes")
    t.add_row("4", "custom", "v?.?.?", "Enter manually")

    console.print(Padding(t, (0, 2)))

    choice = Prompt.ask(
        f"  [{ACCENT}]>[/{ACCENT}]",
        choices=["1", "2", "3", "4"],
        default="1",
    )

    if choice == "1":
        new_ver = patch
    elif choice == "2":
        new_ver = minor
    elif choice == "3":
        new_ver = major
    else:
        raw = Prompt.ask("  Custom version (e.g. 1.2.3)")
        raw = raw.lstrip("v")
        if not re.match(r"^\d+\.\d+\.\d+$", raw):
            fail(f"Invalid version format: {raw}")
        new_ver = Version.parse(raw)

    console.print()
    ok(f"Releasing [{ACCENT}]{new_ver.tag()}[/{ACCENT}]")
    console.print()
    return new_ver, tag


# ═══════════════════════════════════════════════════════════════════════════
# Changelog preview
# ═══════════════════════════════════════════════════════════════════════════


def show_changelog(new_ver: Version, last_tag: Optional[str], n_steps: int):
    step(3, n_steps, "Changelog preview")

    commits = commits_since(last_tag)
    if not commits:
        warn("No commits since last tag. You may be re-releasing the same code.")
        console.print()
        return

    # Categorise by conventional commit prefix
    features, fixes, chores, others = [], [], [], []
    for c in commits:
        lo = c.lower()
        if lo.startswith(("feat", "feature")):
            features.append(c)
        elif lo.startswith(("fix", "bug")):
            fixes.append(c)
        elif lo.startswith(("chore", "ci", "doc")):
            chores.append(c)
        else:
            others.append(c)

    lines = Text()

    def section(icon, title, items, colour):
        if not items:
            return
        lines.append(f"\n  {icon}  {title}\n", style=f"bold {colour}")
        for item in items:
            sha, _, msg = item.partition(" ")
            lines.append(f"   [{DIM}]{sha}[/{DIM}] {msg}\n")

    section("✨", "Features", features, "green")
    section("🐛", "Bug Fixes", fixes, "yellow")
    section("🔧", "Chores / CI", chores, DIM)
    section("📝", "Other", others, "white")

    since = f"since {last_tag}" if last_tag else "all commits"
    console.print(
        Panel(
            lines,
            title=f"[{DIM}]{len(commits)} commits {since}[/{DIM}]",
            border_style=DIM,
            padding=(0, 1),
        )
    )
    console.print()


# ═══════════════════════════════════════════════════════════════════════════
# Linting
# ═══════════════════════════════════════════════════════════════════════════


def maybe_run_lint(n_steps: int, cur_step: int) -> int:
    step(cur_step, n_steps, "Run linting (ruff)")

    ruff = find_tool("ruff")
    if not ruff:
        warn("ruff not found — skipping.")
        console.print()
        return cur_step + 1

    if run_step("Ruff check", ruff, "check", "."):
        console.print()

    return cur_step + 1


def maybe_run_format(n_steps: int, cur_step: int) -> int:
    step(cur_step, n_steps, "Run formatting check (ruff)")

    ruff = find_tool("ruff")
    if not ruff:
        warn("ruff not found — skipping.")
        console.print()
        return cur_step + 1

    if run_step("Ruff format check", ruff, "format", "--check", "."):
        console.print()

    return cur_step + 1


# ═══════════════════════════════════════════════════════════════════════════
# Tests
# ═══════════════════════════════════════════════════════════════════════════


def maybe_run_tests(n_steps: int, cur_step: int) -> int:
    step(cur_step, n_steps, "Run tests")

    if not Confirm.ask("  Run test suite before releasing?", default=True):
        warn("Skipping tests.")
        console.print()
        return cur_step + 1

    pytest = find_tool("pytest")
    if not pytest:
        warn("pytest not found — skipping.")
        console.print()
        return cur_step + 1

    console.print()
    result = subprocess.run(
        [pytest, "--tb=short", "-q"],
        cwd=REPO_ROOT,
    )
    console.print()

    if result.returncode != 0:
        if not Confirm.ask(
            "  [yellow]Tests failed. Release anyway?[/yellow]", default=False
        ):
            console.print("[dim]  Aborted.[/dim]")
            sys.exit(0)
        warn("Releasing despite failing tests.")
    else:
        ok("All tests passed")

    console.print()
    return cur_step + 1


# ═══════════════════════════════════════════════════════════════════════════
# Tag & push
# ═══════════════════════════════════════════════════════════════════════════


def tag_and_push(new_ver: Version, n_steps: int, cur_step: int):
    step(cur_step, n_steps, f"Tag [bold cyan]{new_ver.tag()}[/bold cyan] and push")
    console.print()

    # Confirm before committing
    url = remote_url()
    actions_url = f"{url}/actions" if url else ""

    t = Table(box=box.ROUNDED, border_style=DIM, show_header=False, padding=(0, 2))
    t.add_column("k", style=DIM, width=18)
    t.add_column("v", style="bold white")
    t.add_row("Tag", new_ver.tag())
    t.add_row("Branch", current_branch())
    t.add_row("Remote", url or "[dim]unknown[/dim]")
    t.add_row("Will trigger", "GitHub Actions release workflow")
    console.print(Padding(t, (0, 2)))
    console.print()

    if not Confirm.ask(
        f"  [{ACCENT}]Push {new_ver.tag()} and trigger release?[/{ACCENT}]",
        default=True,
    ):
        console.print("[dim]  Aborted.[/dim]")
        sys.exit(0)

    console.print()

    # Create annotated tag
    run_step(
        f"Create annotated tag {new_ver.tag()}",
        "git",
        "tag",
        "-a",
        new_ver.tag(),
        "-m",
        f"Release {new_ver.tag()}",
    )

    # Push commits first
    branch = current_branch()
    if has_upstream():
        run_step(f"Push branch {branch}", "git", "push", "origin", branch)
    else:
        run_step(
            f"Push branch {branch} (set upstream)",
            "git",
            "push",
            "--set-upstream",
            "origin",
            branch,
        )

    # Push tag
    run_step(f"Push tag {new_ver.tag()}", "git", "push", "origin", new_ver.tag())

    console.print()
    return actions_url


# ═══════════════════════════════════════════════════════════════════════════
# Final summary
# ═══════════════════════════════════════════════════════════════════════════


def summary(new_ver: Version, actions_url: str):
    console.print(Rule(style=ACCENT))
    console.print()

    lines = Text()
    lines.append(
        f"  🎉  Loom {new_ver.tag()} is on its way!\n\n", style=f"bold {ACCENT}"
    )
    lines.append("  GitHub Actions is now:\n", style="white")
    lines.append("    1. Building loom.exe          ", style=DIM)
    lines.append("Windows\n", style="cyan")
    lines.append("    2. Building loom (arm64)      ", style=DIM)
    lines.append("macOS Apple Silicon\n", style="cyan")
    lines.append("    3. Building loom (x86_64)     ", style=DIM)
    lines.append("macOS Intel\n", style="cyan")
    lines.append("    4. Building loom              ", style=DIM)
    lines.append("Linux\n", style="cyan")
    lines.append("    5. Publishing to PyPI         ", style=DIM)
    lines.append("pipx install loomcli\n", style="green")

    if actions_url:
        lines.append(
            f"\n  Monitor live →  [link={actions_url}]{actions_url}[/link]\n",
            style="bold white",
        )

    console.print(Panel(lines, border_style=ACCENT, padding=(0, 1)))
    console.print()

    if actions_url and Confirm.ask("  Open GitHub Actions in browser?", default=True):
        webbrowser.open(actions_url)


# ═══════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════


def main():
    os.chdir(REPO_ROOT)

    N_STEPS = 7  # preflight, version, changelog, lint, format, tests, tag+push

    header()

    preflight(N_STEPS)
    new_ver, last_tag = pick_version(N_STEPS)
    show_changelog(new_ver, last_tag, N_STEPS)
    cur = maybe_run_lint(N_STEPS, 4)
    cur = maybe_run_format(N_STEPS, cur)
    cur = maybe_run_tests(N_STEPS, cur)
    actions_url = tag_and_push(new_ver, N_STEPS, cur)
    summary(new_ver, actions_url)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        console.print("\n[dim]  Cancelled.[/dim]")
        sys.exit(0)
