# Player-applied keybindings

Source-only executor; not packaged, exposed through MCP or live-qualified. Retail is explicit; Classic and Forever remain unimplemented. A binding change configures a key and never invokes its action.

Blizzard's [shared keybinding UI source](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_Settings_Shared/Blizzard_Keybindings.lua) uses the action's binding context with SetBinding/GetBindingAction and commits through SaveBindings(GetCurrentBindingSet()). [Generated documentation](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/KeyBindingsDocumentation.lua) defines binding sets and contexts. Retrieved 2026-11-03. Context must be preserved, including housing modes; never infer that a key has one global meaning.

`BindingOperations.lua` changes one key in an explicit context and current account/character binding set. It refuses combat, a different interface, stale key contents, an unregistered action or a mismatched action context before writes. Empty desired action removes a binding. An unchanged binding is a no-op. SaveBindings is requested after active binding readback; this is not independent proof of disk persistence. Any error after SetBinding may leave an active change, so the eventual import caller must record outcome_unknown before invoking it and must not replay automatically.

Lua5.1 fixtures exercise set, unbind, no-op, stale values, set/context mismatches, unknown actions and combat refusal. Still required: context-aware export, compiler/import format, player review/apply integration, receipt enum support, packaging and live qualification. Macro live-test request remains pending independently.
