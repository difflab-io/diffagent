# Diffagent v0 bot console prototype

Open [index.html](index.html) in a browser. The prototype models declarative bots running on a local harness (`~/.diffagent/bots/{name}`) or a remote Diffagent server, an avatar-only left rail modeled on the Grok Bot macOS desktop app, centered `Chat`, `Stats`, `Sessions`, and `Traces` tabs, a right-hand bot settings sidebar (including directory skills), and a square-tile Connectors hub. All messages, tool calls, traces, and saves are simulated in memory. Reloading resets the sample data.

The format matches Difflab's `designs/prototypes/difflab-v0` and LGTM's `designs/prototypes/lgtm-v0`. It uses pinned CDN imports for Tailwind CSS 4.3.3, Preline UI 5.0.0, and DOMPurify 3.4.16. The HTML requires internet access and no build step. The snapshots are offline-viewable 1120 x 640 PNGs.

The [core concepts PRD](../../../prd/0001-core-concepts/README.md) defines production requirements for the declarative bot console.

## Screens and snapshots

| State                       | Interactive view                              | Snapshot                                                             |
| --------------------------- | --------------------------------------------- | -------------------------------------------------------------------- |
| First-run empty state       | [Empty home](index.html#empty)                | [empty.png](snapshots/empty.png)                                     |
| Bot chat (local harness)    | [Bot chat](index.html#chat)                   | [bot-chat.png](snapshots/bot-chat.png)                               |
| Bot chat (remote server)    | [Remote bot chat](index.html#chat-remote)     | [bot-chat-remote.png](snapshots/bot-chat-remote.png)                 |
| Inline tool approval gate   | [Chat approval](index.html#chat-approval)     | [bot-chat-approval.png](snapshots/bot-chat-approval.png)             |
| Create declarative bot      | [Create bot](index.html#new-bot)              | [bot-create.png](snapshots/bot-create.png)                           |
| Bot statistics dashboard    | [Stats](index.html#stats)                     | [bot-stats.png](snapshots/bot-stats.png)                             |
| Multi-channel sessions      | [Sessions](index.html#sessions)               | [bot-sessions.png](snapshots/bot-sessions.png)                       |
| Request traces table        | [Traces](index.html#traces)                   | [bot-traces.png](snapshots/bot-traces.png)                           |
| Trace waterfall detail      | [Trace detail](index.html#trace-detail)       | [bot-trace-waterfall.png](snapshots/bot-trace-waterfall.png)         |
| Right sidebar: General      | [Bot settings](index.html#bot-settings)       | [bot-settings-general.png](snapshots/bot-settings-general.png)       |
| Right sidebar: Skills       | [Bot skills](index.html#bot-skills)           | [bot-settings-skills.png](snapshots/bot-settings-skills.png)         |
| Right sidebar: Connectors   | [Bot connectors](index.html#bot-connectors)   | [bot-settings-connectors.png](snapshots/bot-settings-connectors.png) |
| Right sidebar: Channels     | [Bot channels](index.html#bot-channels)       | [bot-settings-channels.png](snapshots/bot-settings-channels.png)     |
| Right sidebar: Files        | [Declarative files](index.html#bot-files)     | [bot-settings-files.png](snapshots/bot-settings-files.png)           |
| Connectors square tile grid | [User connectors](index.html#user-connectors) | [user-connectors.png](snapshots/user-connectors.png)                 |
| Add custom MCP server       | [Add MCP](index.html#add-mcp)                 | [user-connectors-mcp.png](snapshots/user-connectors-mcp.png)         |
| User avatar menu            | [Account menu](index.html#user-menu)          | [user-account-menu.png](snapshots/user-account-menu.png)             |
| User settings and servers   | [User settings](index.html#user-settings)     | [user-settings.png](snapshots/user-settings.png)                     |

## Console behavior

The full-width top bar places the Diffagent logo and active bot identity on the left, centers the four workspace tabs (`Chat`, `Stats`, `Sessions`, `Traces`), and puts the `Settings` button on the far right to toggle a right-hand bot settings sidebar. There is no bottom footer bar and no `New chat` button: each bot has one persistent desktop chat stream.

The left navigation rail runs top-to-bottom without collapse or search, showing bot avatars and a dashed `+` slot to create a new bot. At the bottom of the rail sits a four-cube apps icon for the square-tile Connectors hub above the user avatar menu (settings, remote servers, log out).

Creating a new bot asks only for name, avatar shape/color, and local vs. remote harness; the bot directory (`~/.diffagent/bots/{slug}`) is automatic, and skills (`skills/{name}/SKILL.md`), connectors, and channels are configured afterward in the right sidebar.

## Refresh snapshots and check interactions

```sh
DIFFAGENT_BROWSER_CHANNEL=chrome \
node designs/prototypes/diffagent-v0/capture.mjs
```

Refresh the snapshots after changing `index.html`.
