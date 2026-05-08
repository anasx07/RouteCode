import asyncio
from typing import List, Dict, Any, TYPE_CHECKING
from .events import bus
from .state import count_tokens

if TYPE_CHECKING:
    from .context import LoomContext
    from .history import ConversationHistory


class ContextManager:
    """
    Consolidated service for context compaction and summarization.
    """

    def __init__(self, ctx: "LoomContext"):
        self.ctx = ctx
        self.micro_threshold = 70.0
        self.full_threshold = 85.0

    def check_and_compact(self, history: "ConversationHistory", model: str) -> bool:
        """
        Checks context usage and performs automatic micro-compaction if needed.
        Returns True if compaction was performed.
        """
        usage_pct = self.ctx.state.get_context_usage(model)

        if usage_pct > self.full_threshold:
            bus.emit("context.threshold_warning", usage=usage_pct, type="full")
        elif usage_pct > self.micro_threshold:
            if not self.ctx.state.context_warned:
                bus.emit("context.threshold_warning", usage=usage_pct, type="micro")

            # Auto micro-compact
            original_len = len(history)
            compacted = self.microcompact(history.get_messages())
            if len(compacted) < original_len:
                history.set_messages(compacted)
                # Recalculate tokens for the retained history
                retained_content = " ".join(
                    m.get("content", "") or "" for m in compacted
                )
                if hasattr(self.ctx.state, "_tokenizer") and self.ctx.state._tokenizer:
                    new_token_count = self.ctx.state._tokenizer.count_tokens(
                        retained_content, model
                    )
                    self.ctx.state._tokenizer.load_state(
                        new_token_count, self.ctx.state.estimated_cost
                    )
                    self.ctx.state.tokens_used = new_token_count
                else:
                    self.ctx.state.tokens_used = count_tokens(retained_content, model)

                self.ctx.state.reset_context_warning()
                bus.emit(
                    "context.compacted",
                    type="micro",
                    saved=original_len - len(compacted),
                )
                return True

        return False

    def microcompact(self, messages: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Strip old tool results without an API call while preserving critical context.
        """
        if len(messages) < 6:
            return messages

        system_msg = messages[0]

        turns = []
        current_turn = []
        last_role = None
        for msg in messages[1:]:
            role = msg.get("role")
            if role == "user":
                if current_turn:
                    turns.append(current_turn)
                current_turn = [msg]
            elif role == "assistant":
                if last_role in ("tool", "assistant"):
                    if current_turn:
                        turns.append(current_turn)
                    current_turn = [msg]
                else:
                    current_turn.append(msg)
            else:
                current_turn.append(msg)
            last_role = role

        if current_turn:
            turns.append(current_turn)

        kept = [system_msg]
        for i, turn in enumerate(turns):
            is_recent = i >= len(turns) - 4
            has_critical_context = False
            for msg in turn:
                if msg.get("role") == "tool" and msg.get("name") in (
                    "file_read",
                    "file_edit",
                    "file_write",
                    "task",
                ):
                    has_critical_context = True
                    break

            if is_recent or has_critical_context:
                kept.extend(turn)

        return kept

    async def summarize_compact(self, history: "ConversationHistory") -> bool:
        """
        Performs full summarization of old history using a sub-agent.
        """
        messages = history.get_messages()
        if len(messages) < 4:
            return False

        keep_count = 7
        to_compact = messages[1:-keep_count] if len(messages) > keep_count + 1 else []
        if len(to_compact) < 2:
            return False

        summary_text = "\n".join(
            f"[{m.get('role', '?')}]: {(m.get('content', '') or '')[:200]}"
            for m in to_compact
        )

        prev_summary = ""
        for m in reversed(messages):
            if m.get("role") == "system" and "[Compacted]" in str(m.get("content", "")):
                prev_summary = str(m.get("content", ""))
                break

        anchor = (
            f"\nPrevious summary (update this):\n{prev_summary}" if prev_summary else ""
        )

        compact_prompt = (
            "Summarize this conversation. Format as Markdown headers (Goal, Constraints, Progress, Decisions, Next Steps, Critical Context, Relevant Files).\n"
            f"{anchor}\n\n"
            "Conversation to summarize:\n" + summary_text
        )

        try:
            from ..tools.task import _run_sub_agent_async
            from ..task_manager import task_manager

            task_id = f"c{abs(hash(compact_prompt)) % 10**7}"
            task_manager.create("Context compaction", None, task_id)

            # Run as async task on the main loop
            asyncio.create_task(
                _run_sub_agent_async(compact_prompt, 3, task_id, self.ctx)
            )
            return True
        except Exception:
            return False
