import os
from typing import List
from .. import ui as _ui
from ..ui import print_success, print_error, RouteCodeDialog
from ..core import RouteCodeContext
from ..agents.registry import PROVIDER_MAP

PROVIDER_LIST = list(PROVIDER_MAP.keys())


async def handle_provider(args: List[str], ctx: RouteCodeContext):
    # Load models database to get provider names
    import json
    from pathlib import Path

    models_db_path = Path(__file__).parent.parent / "models_api.json"
    try:
        with open(models_db_path, "r", encoding="utf-8") as f:
            models_db = json.load(f)
    except Exception:
        models_db = {}

    popular_providers = {
        "opencode": "(Recommended)",
        "opencode-go": "Low cost subscription for everyone",
        "openai": "(ChatGPT Plus/Pro or API key)",
        "github": "",
        "anthropic": "(API key)",
        "google": "",
    }

    from ..ui import ModelPaletteMenu

    while True:
        values = []

        # Popular category
        values.append((None, "Popular", True, None, None))
        for p_id, desc in popular_providers.items():
            if p_id in PROVIDER_LIST:
                p_info = models_db.get(p_id, {})
                name = p_info.get("name", p_id.capitalize())
                # Check if connected
                is_connected = bool(ctx.config.get_api_key(p_id))
                if not is_connected and "env" in p_info:
                    for env_var in p_info["env"]:
                        if os.environ.get(env_var):
                            is_connected = True
                            break

                label = f"✓ {name}" if is_connected else f"  {name}"
                values.append((p_id, label, False, desc, None))

        # Other category
        values.append((None, "Other", True, None, None))
        for p_id in PROVIDER_LIST:
            if p_id in popular_providers:
                continue
            p_info = models_db.get(p_id, {})
            name = p_info.get("name", p_id.capitalize())

            is_connected = bool(ctx.config.get_api_key(p_id))
            if not is_connected and "env" in p_info:
                for env_var in p_info["env"]:
                    if os.environ.get(env_var):
                        is_connected = True
                        break

            label = f"✓ {name}" if is_connected else f"  {name}"
            values.append((p_id, label, False, None, None))

        menu = ModelPaletteMenu(
            title="Connect a provider", values=values, active_value=ctx.config.provider
        )
        menu.show_footer = False
        result = await menu.run_async()

        if not result:
            break

        # Check for API key
        is_connected = bool(ctx.config.get_api_key(result))
        if not is_connected:
            p_info = models_db.get(result, {})
            if "env" in p_info:
                for env_var in p_info["env"]:
                    if os.environ.get(env_var):
                        is_connected = True
                        break

        if not is_connected:
            new_key = await RouteCodeDialog(
                title=f"Setup {result.capitalize()}",
                text=_ui.get_dialog_text(
                    f"Paste your {result} API key here and press Enter:", "input"
                ),
                password=True,
                dialog_type="input",
            ).run_async()
            if new_key:
                ctx.config.set_api_key(result, new_key)
                await ctx.config.save_async()
                print_success(
                    f"API key for {result} has been saved! You can now select its models in the model menu."
                )
                break
            else:
                continue
        else:
            action = await RouteCodeDialog(
                title=f"Update {result.capitalize()}",
                text=_ui.get_dialog_text(
                    f"{result} is already connected. Do you want to update the API key?",
                    "button",
                ),
                buttons=[("Update", True), ("Back", False)],
                dialog_type="button",
            ).run_async()

            if action:
                new_key = await RouteCodeDialog(
                    title=f"Update {result.capitalize()}",
                    text=_ui.get_dialog_text(
                        f"Paste your new {result} API key:", "input"
                    ),
                    password=True,
                    dialog_type="input",
                ).run_async()
                if new_key:
                    ctx.config.set_api_key(result, new_key)
                    await ctx.config.save_async()
                    print_success(f"API key for {result} has been updated.")
                    break
                else:
                    continue
            else:
                continue


async def handle_model(args: List[str], ctx: RouteCodeContext):
    if args:
        if ":" in args[0]:
            p, m = args[0].split(":", 1)
            ctx.config.provider = p
            ctx.config.model = m
        else:
            ctx.config.model = args[0]
        await ctx.config.save_async()
        ctx.console.print(
            f"Model set to: [bold green]{ctx.config.model}[/bold green] ([dim]{ctx.config.provider}[/dim])"
        )
        return

    # Load models database
    import json
    from pathlib import Path

    models_db_path = Path(__file__).parent.parent / "models_api.json"
    try:
        with open(models_db_path, "r", encoding="utf-8") as f:
            models_db = json.load(f)
    except Exception:
        models_db = {}

    async def _build_values():
        values = []

        # Determine which providers are connected
        connected_providers = []
        for p_id, p_info in models_db.items():
            is_conn = False
            if ctx.config.get_api_key(p_id):
                is_conn = True
            else:
                env_vars = p_info.get("env", [])
                for env_var in env_vars:
                    if os.environ.get(env_var):
                        is_conn = True
                        break
            if is_conn:
                connected_providers.append(p_id)

        # Pre-fetch models for connected providers (this uses our live logic)
        provider_models = {}
        for p_id in connected_providers:
            try:
                provider_cls = PROVIDER_MAP.get(p_id)
                if provider_cls:
                    # Create provider instance
                    api_key = ctx.config.get_api_key(p_id) or ""
                    provider = provider_cls(api_key)
                    # This will try live fetch then fallback to JSON models
                    live_list = await provider.get_models()
                    provider_models[p_id] = live_list
            except Exception:
                pass

        favorites = ctx.config.favorites
        recent = ctx.config.recent_models

        # Helper to format a model item
        def format_item(p, m_id, name=None):
            p_info = models_db.get(p, {})
            m_info = p_info.get("models", {}).get(m_id, {})
            display_name = name or m_info.get("name", m_id)
            label = (
                f"★ {display_name}" if [p, m_id] in favorites else f"  {display_name}"
            )

            # Check cost from metadata
            cost = m_info.get("cost", {})
            is_free_meta = cost.get("input") == 0 and cost.get("output") == 0

            # Auto-detect free from name/id if not in meta
            is_free_auto = "free" in m_id.lower() or "free" in display_name.lower()

            badge = "Free" if (is_free_meta or is_free_auto) else None
            return (f"{p}:{m_id}", label, False, p_info.get("name", p), badge)

        # 1. Recent
        if recent:
            recent_items = []
            for p, m in recent:
                if p not in provider_models:
                    continue
                if [p, m] in favorites:
                    continue
                recent_items.append(format_item(p, m))
            if recent_items:
                values.append((None, "Recent", True, None, None))
                values.extend(recent_items)

        # 2. Favorites
        if favorites:
            fav_items = []
            for p, m in favorites:
                if p not in provider_models:
                    continue
                fav_items.append(format_item(p, m))
            if fav_items:
                values.append((None, "Favorites", True, None, None))
                values.extend(fav_items)

        # 3. All connected providers
        curated = [
            "opencode",
            "opencode-go",
            "cloudflare",
            "nvidia",
            "google",
            "openai",
            "anthropic",
        ]
        sorted_pids = sorted(
            provider_models.keys(),
            key=lambda x: (x not in curated, curated.index(x) if x in curated else x),
        )

        for p_id in sorted_pids:
            p_info = models_db.get(p_id, {})
            values.append((None, p_info.get("name", p_id), True, None, None))
            for m in provider_models[p_id]:
                m_id = m["id"]
                name = m["name"]
                values.append(format_item(p_id, m_id, name))

        return values

    values = await _build_values()

    from ..ui import ModelPaletteMenu

    active_val = f"{ctx.config.provider}:{ctx.config.model}"
    menu = ModelPaletteMenu(
        title="Select model", values=values, active_value=active_val
    )

    def on_favorite(val):
        if val and ":" in val:
            p, m = val.split(":", 1)
            ctx.config.toggle_favorite(p, m)
            return _build_values()

    menu.on_favorite = on_favorite

    def on_connect_provider_stub():
        pass

    menu.on_connect_provider = on_connect_provider_stub

    result = await menu.run_async()

    if result == "__connect_provider__":
        await handle_provider([], ctx)
    elif result:
        p, m = result.split(":", 1)
        ctx.config.provider = p
        ctx.config.model = m
        await ctx.config.save_async()
        ctx.console.print(
            f"\n[success]✔[/success] Model set to [bold cyan]{m}[/bold cyan] ([dim]{p}[/dim])"
        )


async def handle_config(args: List[str], ctx: RouteCodeContext):
    action = await RouteCodeDialog(
        title="routecode Configuration",
        text=_ui.get_dialog_text("What would you like to do?", "button"),
        buttons=[
            ("View", "view"),
            ("Update Key", "update"),
            ("Delete Key", "delete"),
            ("Back", "back"),
        ],
        dialog_type="button",
    ).run_async()

    if action == "view":
        from rich.table import Table

        table = Table(
            title="Current Configuration", show_header=True, header_style="bold magenta"
        )
        table.add_column("Key")
        table.add_column("Value")
        table.add_row("Provider", ctx.config.provider)
        table.add_row("Model", ctx.config.model)
        for p, key in ctx.config.api_keys.items():
            masked_key = (
                key[:4] + "*" * (max(0, len(key) - 8)) + key[-4:]
                if len(key) > 8
                else "****"
            )
            table.add_row(f"{p} API Key", masked_key)
        ctx.console.print(table)

    elif action == "update":
        p_to_update = await RouteCodeDialog(
            title="Update API Key",
            text=_ui.get_dialog_text(
                "Select which provider's key you want to update:", "radio"
            ),
            values=[(p, p.capitalize()) for p in PROVIDER_LIST],
            dialog_type="radio",
        ).run_async()
        if p_to_update:
            new_key = await RouteCodeDialog(
                title=f"Update {p_to_update.capitalize()}",
                text=_ui.get_dialog_text(
                    f"Paste your new {p_to_update} API key:", "input"
                ),
                password=True,
                dialog_type="input",
            ).run_async()
            if new_key:
                ctx.config.set_api_key(p_to_update, new_key)
                await ctx.config.save_async()
                print_success(f"Key for {p_to_update} has been replaced.")

    elif action == "delete":
        existing_keys = list(ctx.config.api_keys.keys())
        if not existing_keys:
            await RouteCodeDialog(
                title="Error",
                text=_ui.get_dialog_text("No API keys found to delete.", "message"),
                dialog_type="message",
            ).run_async()
            return

        p_to_delete = await RouteCodeDialog(
            title="Delete API Key",
            text=_ui.get_dialog_text(
                "Select which provider's key you want to remove:", "radio"
            ),
            values=[(p, p.capitalize()) for p in existing_keys],
            dialog_type="radio",
        ).run_async()
        if p_to_delete:
            confirm = await RouteCodeDialog(
                title="Confirm Deletion",
                text=_ui.get_dialog_text(
                    f"Are you sure you want to delete the {p_to_delete} key?", "button"
                ),
                buttons=[("Yes", True), ("No", False)],
                dialog_type="button",
            ).run_async()
            if confirm:
                del ctx.config.api_keys[p_to_delete]
                await ctx.config.save_async()
                print_success(f"Key for {p_to_delete} has been deleted.")


async def handle_theme(args: List[str], ctx: RouteCodeContext):
    from ..ui import THEMES, apply_theme
    from .core import _refresh_screen

    if args:
        name = args[0]
        if name in THEMES:
            apply_theme(name)
            ctx.config.theme = name
            await ctx.config.save_async()
            _refresh_screen(ctx)
            print_success(f"Theme set to: {name}")
        else:
            avail = ", ".join(THEMES.keys())
            print_error(f"Theme '{name}' not found. Available: {avail}")
        return

    active = ctx.config.theme
    choices = [(n, n.capitalize()) for n in THEMES]

    from ..ui import PaletteMenu, apply_theme

    original_theme = ctx.config.theme

    def on_theme_hover(theme_name):
        apply_theme(theme_name)

    result = await PaletteMenu(
        title="Themes", values=choices, active_value=active, on_hover=on_theme_hover
    ).run_async()

    if result:
        apply_theme(result)
        ctx.config.theme = result
        await ctx.config.save_async()
        _refresh_screen(ctx)
        print_success(f"Theme set to: {result}")
    else:
        apply_theme(original_theme)


async def handle_personality(args: List[str], ctx: RouteCodeContext):
    from ..domain.personalities import load_personalities, get_active_personality

    if args:
        name = args[0]
        personalities = load_personalities()
        if name in personalities:
            ctx.config.personality = name
            await ctx.config.save_async()
            print_success(f"Personality set to: {name}")
        else:
            avail = ", ".join(personalities.keys())
            print_error(f"Personality '{name}' not found. Available: {avail}")
        return

    personalities = load_personalities()
    active = get_active_personality()
    choices = [(n, f"{n}: {p.description}") for n, p in personalities.items()]

    result = await RouteCodeDialog(
        title="Select Personality",
        text=_ui.get_dialog_text(
            f"Current: {active.name} ({active.description})", "radio"
        ),
        values=choices,
        dialog_type="radio",
    ).run_async()

    if result:
        ctx.config.personality = result
        await ctx.config.save_async()
        print_success(f"Personality set to: {result}")
