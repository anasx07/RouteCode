import os
import inspect
from typing import List
from .. import ui as _ui
from ..ui import print_success, print_error, print_step, LoomDialog
from ..core import LoomContext
from ..agents.registry import PROVIDER_MAP

PROVIDER_LIST = list(PROVIDER_MAP.keys())

async def handle_provider(args: List[str], ctx: LoomContext):
    result = await LoomDialog(
        title="Select AI Provider",
        text=_ui.get_dialog_text("Choose the provider you want to use for this session:", "radio"),
        values=[(p, p.capitalize()) for p in PROVIDER_LIST],
        dialog_type="radio"
    ).run_async()

    if result:
        ctx.config.provider = result
        await ctx.config.save_async()
        ctx.console.print(f"\n[success]✔[/success] Provider switched to [bold cyan]{result}[/bold cyan]")
        
        # Check for API key
        if not ctx.config.get_api_key(result):
            new_key = await LoomDialog(
                title=f"Setup {result.capitalize()}",
                text=_ui.get_dialog_text(f"No API key found for {result}. Please paste it here and press Enter:", "input"),
                password=True,
                dialog_type="input"
            ).run_async()
            if new_key:
                ctx.config.set_api_key(result, new_key)
                await ctx.config.save_async()
                print_success(f"API key for {result} has been saved.")

async def handle_model(args: List[str], ctx: LoomContext):
    if args:
        if ":" in args[0]:
            p, m = args[0].split(":", 1)
            ctx.config.provider = p
            ctx.config.model = m
        else:
            ctx.config.model = args[0]
        await ctx.config.save_async()
        ctx.console.print(f"Model set to: [bold green]{ctx.config.model}[/bold green] ([dim]{ctx.config.provider}[/dim])")
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

    # Structure data for CategoryRadioList
    # (value, label, is_header, description, tag)
    values = []
    
    # Determine which providers are connected (have an API key)
    connected_providers = set()
    for p_id, p_info in models_db.items():
        # Check if we have an API key stored for this provider
        if ctx.config.get_api_key(p_id):
            connected_providers.add(p_id)
            continue
        # Also check environment variables listed in the provider's 'env' field
        env_vars = p_info.get("env", [])
        for env_var in env_vars:
            if os.environ.get(env_var):
                connected_providers.add(p_id)
                break

    # 1. Favorites (only from connected providers)
    favorites = ctx.config.favorites
    if favorites:
        fav_items = []
        for p, m in favorites:
            if p not in connected_providers: continue
            p_info = models_db.get(p, {})
            m_info = p_info.get("models", {}).get(m, {})
            name = m_info.get("name", m)
            fav_items.append((f"{p}:{m}", name, False, p_info.get("name", p), "Free" if "free" in name.lower() else None))
        if fav_items:
            values.append((None, "Favorites", True, None, None))
            values.extend(fav_items)

    # 2. Recent (only from connected providers)
    recent = ctx.config.recent_models
    if recent:
        recent_items = []
        for p, m in recent:
            if p not in connected_providers: continue
            if [p, m] in favorites: continue
            p_info = models_db.get(p, {})
            m_info = p_info.get("models", {}).get(m, {})
            name = m_info.get("name", m)
            recent_items.append((f"{p}:{m}", name, False, p_info.get("name", p), "Free" if "free" in name.lower() else None))
        if recent_items:
            values.append((None, "Recent", True, None, None))
            values.extend(recent_items)

    # 3. Connected Providers - curated order first
    curated = ["opencode", "opencode-go", "cloudflare", "nvidia", "google", "openai", "anthropic"]
    for p_id in curated:
        if p_id not in connected_providers: continue
        p_info = models_db.get(p_id)
        if not p_info: continue
        p_models = p_info.get("models", {})
        if not p_models: continue
        
        values.append((None, p_info.get("name", p_id), True, None, None))
        for m_id, m_info in p_models.items():
            name = m_info.get("name", m_id)
            values.append((f"{p_id}:{m_id}", name, False, None, "Free" if "free" in name.lower() else None))

    # 4. All other connected providers
    for p_id, p_info in models_db.items():
        if p_id in curated: continue
        if p_id not in connected_providers: continue
        p_models = p_info.get("models", {})
        if not p_models: continue
        
        values.append((None, p_info.get("name", p_id), True, None, None))
        for m_id, m_info in p_models.items():
            name = m_info.get("name", m_id)
            values.append((f"{p_id}:{m_id}", name, False, None, "Free" if "free" in name.lower() else None))

    from ..ui import ModelPaletteMenu
    
    active_val = f"{ctx.config.provider}:{ctx.config.model}"
    menu = ModelPaletteMenu(title="Select model", values=values, active_value=active_val)
    
    def on_favorite(val):
        if val and ":" in val:
            p, m = val.split(":", 1)
            ctx.config.toggle_favorite(p, m)
    menu.on_favorite = on_favorite
    
    def on_connect_provider_stub(): pass 
    menu.on_connect_provider = on_connect_provider_stub
    
    result = await menu.run_async()
    
    if result == "__connect_provider__":
        await handle_provider([], ctx)
    elif result:
        p, m = result.split(":", 1)
        ctx.config.provider = p
        ctx.config.model = m
        await ctx.config.save_async()
        ctx.console.print(f"\n[success]✔[/success] Model set to [bold cyan]{m}[/bold cyan] ([dim]{p}[/dim])")

async def handle_config(args: List[str], ctx: LoomContext):
    action = await LoomDialog(
        title="LoomCLI Configuration",
        text=_ui.get_dialog_text("What would you like to do?", "button"),
        buttons=[
            ("View", "view"),
            ("Update Key", "update"),
            ("Delete Key", "delete"),
            ("Back", "back")
        ],
        dialog_type="button"
    ).run_async()

    if action == "view":
        from rich.table import Table
        table = Table(title="Current Configuration", show_header=True, header_style="bold magenta")
        table.add_column("Key")
        table.add_column("Value")
        table.add_row("Provider", ctx.config.provider)
        table.add_row("Model", ctx.config.model)
        for p, key in ctx.config.api_keys.items():
            masked_key = key[:4] + "*" * (max(0, len(key) - 8)) + key[-4:] if len(key) > 8 else "****"
            table.add_row(f"{p} API Key", masked_key)
        ctx.console.print(table)

    elif action == "update":
        p_to_update = await LoomDialog(
            title="Update API Key",
            text=_ui.get_dialog_text("Select which provider's key you want to update:", "radio"),
            values=[(p, p.capitalize()) for p in PROVIDER_LIST],
            dialog_type="radio"
        ).run_async()
        if p_to_update:
            new_key = await LoomDialog(
                title=f"Update {p_to_update.capitalize()}",
                text=_ui.get_dialog_text(f"Paste your new {p_to_update} API key:", "input"),
                password=True,
                dialog_type="input"
            ).run_async()
            if new_key:
                ctx.config.set_api_key(p_to_update, new_key)
                await ctx.config.save_async()
                print_success(f"Key for {p_to_update} has been replaced.")

    elif action == "delete":
        existing_keys = list(ctx.config.api_keys.keys())
        if not existing_keys:
            await LoomDialog(title="Error", text=_ui.get_dialog_text("No API keys found to delete.", "message"), dialog_type="message").run_async()
            return
            
        p_to_delete = await LoomDialog(
            title="Delete API Key",
            text=_ui.get_dialog_text("Select which provider's key you want to remove:", "radio"),
            values=[(p, p.capitalize()) for p in existing_keys],
            dialog_type="radio"
        ).run_async()
        if p_to_delete:
            confirm = await LoomDialog(
                title="Confirm Deletion",
                text=_ui.get_dialog_text(f"Are you sure you want to delete the {p_to_delete} key?", "button"),
                buttons=[("Yes", True), ("No", False)],
                dialog_type="button"
            ).run_async()
            if confirm:
                del ctx.config.api_keys[p_to_delete]
                await ctx.config.save_async()
                print_success(f"Key for {p_to_delete} has been deleted.")

async def handle_theme(args: List[str], ctx: LoomContext):
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
        title="Themes",
        values=choices,
        active_value=active,
        on_hover=on_theme_hover
    ).run_async()

    if result:
        apply_theme(result)
        ctx.config.theme = result
        await ctx.config.save_async()
        _refresh_screen(ctx)
        print_success(f"Theme set to: {result}")
    else:
        apply_theme(original_theme)

async def handle_personality(args: List[str], ctx: LoomContext):
    from ..personalities import load_personalities, get_active_personality

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

    result = await LoomDialog(
        title="Select Personality",
        text=_ui.get_dialog_text(f"Current: {active.name} ({active.description})", "radio"),
        values=choices,
        dialog_type="radio"
    ).run_async()

    if result:
        ctx.config.personality = result
        await ctx.config.save_async()
        print_success(f"Personality set to: {result}")
