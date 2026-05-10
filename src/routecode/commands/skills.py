from typing import List
from ..ui import print_success, print_error, RouteCodeDialog
from ..core import RouteCodeContext
from ..domain.skills import discover_skills, run_skill


async def handle_skill(args: List[str], ctx: RouteCodeContext):
    skills = discover_skills(only_enabled=True)
    if not skills:
        ctx.console.print(
            "[dim]No active skills available. Check /skill-manage to enable some.[/dim]"
        )
        return

    if args:
        skill_name = args[0]
        if skill_name in skills:
            arg_str = " ".join(args[1:])
            ctx.console.print_tool_call(f"Skill({skill_name})", {"args": arg_str})
            result = run_skill(skills[skill_name], arg_str, ctx)
            ctx.console.print_tool_result(result)
            return
        else:
            print_error(f"Skill '{skill_name}' not found.")
            return

    # No args: show interactive picker
    choices = [(name, f"{name}: {s.description}") for name, s in skills.items()]

    result = await RouteCodeDialog(
        title="Invoke Skill",
        text="Select a skill to execute:",
        values=choices,
        dialog_type="radio",
    ).run_async()

    if result:
        # Ask for optional arguments
        arg_str = await RouteCodeDialog(
            title=f"Arguments for {result}",
            text=f"Enter optional arguments for '{result}':",
            dialog_type="input",
        ).run_async()

        arg_str = arg_str or ""
        ctx.console.print_tool_call(f"Skill({result})", {"args": arg_str})
        res = run_skill(skills[result], arg_str, ctx)
        ctx.console.print_tool_result(res)


async def handle_skill_create(args: List[str], ctx: RouteCodeContext):
    name = await RouteCodeDialog(
        title="Create New Skill",
        text="Enter a name for the skill (e.g., 'deploy-lambda'):",
        dialog_type="input",
    ).run_async()
    if not name:
        return

    description = await RouteCodeDialog(
        title="Skill Description",
        text="Enter a short description of what this skill does:",
        dialog_type="input",
    ).run_async()
    if not description:
        return

    prompt = await RouteCodeDialog(
        title="Skill Prompt",
        text="Enter the system prompt or instructions for this skill:",
        dialog_type="input",
    ).run_async()
    if not prompt:
        return

    context = await RouteCodeDialog(
        title="Execution Context",
        text="Select how this skill should run:",
        values=[
            ("inline", "Inline (expands in main chat)"),
            ("fork", "Fork (runs as sub-agent)"),
        ],
        dialog_type="radio",
    ).run_async()
    if not context:
        return

    # Use the tool logic to create it
    from ..tools.skill import SkillCreatorTool

    tool = SkillCreatorTool()
    result = tool._run(
        name=name, description=description, prompt=prompt, context=context
    )

    if result.get("success"):
        print_success(result["message"])
    else:
        print_error(result.get("error", "Failed to create skill."))


def handle_skill_find(args: List[str], ctx: RouteCodeContext):
    from rich.table import Table

    skills = discover_skills()
    if not skills:
        ctx.console.print("[dim]No skills found.[/dim]")
        return

    table = Table(
        title="Available Skills", show_header=True, header_style="bold magenta"
    )
    table.add_column("Name", style="cyan")
    table.add_column("Context", style="green")
    table.add_column("Description")

    for name, s in skills.items():
        status = "[green]Enabled[/green]" if s.enabled else "[red]Disabled[/red]"
        source = "[yellow]Bundled[/yellow]" if s.is_bundled else "[blue]External[/blue]"
        table.add_row(name, s.context, source, status, s.description)

    ctx.console.print(table)


async def handle_skill_manage(args: List[str], ctx: RouteCodeContext):
    from ..config import config

    skills = discover_skills()
    if not skills:
        print_error("No skills found.")
        return

    choices = []
    for name, s in skills.items():
        label = (
            f"{name} ({'Enabled' if s.enabled else 'Disabled'}) - {s.description[:60]}"
        )
        choices.append((name, label))

    result = await RouteCodeDialog(
        title="Manage Skills",
        text="Select a skill to toggle its enabled status:",
        values=choices,
        dialog_type="radio",  # Using radio for single toggle, or I could use checkboxes if I had them
    ).run_async()

    if result:
        if result in config.disabled_skills:
            config.disabled_skills.remove(result)
            print_success(f"Skill '{result}' enabled.")
        else:
            config.disabled_skills.append(result)
            print_success(f"Skill '{result}' disabled.")
        config.save()


async def handle_skill_uninstall(args: List[str], ctx: RouteCodeContext):
    import shutil

    skills = discover_skills()

    if not args:
        # Show a picker for external skills only
        external = [(n, n) for n, s in skills.items() if not s.is_bundled]
        if not external:
            print_error("No external skills found to uninstall.")
            return

        target = await RouteCodeDialog(
            title="Uninstall Skill",
            text="Select an EXTERNAL skill to permanently delete:",
            values=external,
            dialog_type="radio",
        ).run_async()
    else:
        target = args[0]

    if not target or target not in skills:
        if args:
            print_error(f"Skill '{target}' not found.")
        return

    skill = skills[target]
    if skill.is_bundled:
        print_error(
            "Cannot uninstall bundled skills. Use /skill-manage to disable them instead."
        )
        return

    confirm = await RouteCodeDialog(
        title="Confirm Uninstall",
        text=f"Are you sure you want to PERMANENTLY DELETE the skill '{target}'?",
        dialog_type="confirm",
    ).run_async()

    if confirm:
        try:
            # Skills are in folders now
            folder = skill.path.parent
            shutil.rmtree(folder)
            print_success(f"Skill '{target}' uninstalled successfully.")

            # Also remove from disabled list if present
            from ..config import config

            if target in config.disabled_skills:
                config.disabled_skills.remove(target)
                config.save()
        except Exception as e:
            print_error(f"Failed to uninstall skill: {e}")
