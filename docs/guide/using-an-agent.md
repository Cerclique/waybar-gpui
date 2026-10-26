# Using a coding agent

GPUI is pre-1.0, so a coding agent's memory of its API is usually wrong for
the snapshot the SDK pins, and it knows little about `waybar_gpui`. The
repository ships an agent skill,
[`skills/waybar-gpui/SKILL.md`](../../skills/waybar-gpui/SKILL.md), that sets
it straight. It holds no API of its own: it makes the agent find your
project's `waybar_gpui` and GPUI versions, then read the guide, the SDK's
source and GPUI's source for exactly those versions. It also tells it:

- the words to use (panel, bar module, popover…: the
  [glossary](../GLOSSARY.md));
- the steps to add a panel, ending in `preview`;
- to check your Waybar and Hyprland versions before editing their config;
- to warn you before running `preview` or `toggle`, which open popovers on
  your screen.

One file serves every version: an old copy keeps working after you upgrade the
SDK, and the guide for your version wins wherever the two disagree.

## Claude Code

Copy the skill into your program's repository, so everyone working on it gets
it:

```sh
mkdir -p .claude/skills/waybar-gpui
curl -fsSL -o .claude/skills/waybar-gpui/SKILL.md \
  https://raw.githubusercontent.com/Cerclique/waybar-gpui/main/skills/waybar-gpui/SKILL.md
```

or into `~/.claude/skills/waybar-gpui/` for all your projects. Claude Code
uses it on its own when you work on a panel; `/waybar-gpui` invokes it by
hand.

## Other agents

Agents that read skills (`SKILL.md` with a `name` and `description`) take the
same file in their own skills folder. For the rest, save it anywhere in the
repository and point the agent's instructions file at it, e.g. in
`AGENTS.md`:

```markdown
Before working on a panel, read and follow `docs/waybar-gpui-skill.md`.
```
