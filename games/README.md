# Vulcan games (web3d platform)

Games written in Vulcan that run on the web3d platform — a WebGL2 host that
packages a game into one self-contained `index.html` (and an Android WebView
project). Current games: `arcade/` (four mini-games), `vulcancraft/` (1v1 voxel
duel), `obby/` (Roblox-style obstacle course).

## game.json — make the host yours

The host ships with sensible defaults (help tabs, touch layout, launcher icon),
but nothing is forced. If your game folder contains a `game.json`, the packager
injects it and the host merges it over its defaults:

```json
{
  "title": "My Game",
  "first_help": "How to play",
  "help": {
    "How to play": "<h4>How to play</h4><ul><li>...</li></ul>",
    "Touch": "<h4>Touch</h4><ul><li>...</li></ul>"
  },
  "touch": { "sprint": false }
}
```

| key | what it controls |
|---|---|
| `title` | the page title |
| `help` | **replaces the whole help tab set** (name → HTML). `"help": false` removes the help system entirely |
| `first_help` | tab auto-opened on first run (tab name, or `false` to never auto-open) |
| `touch` | touch-control default overrides — same keys the Options > Customize touch controls editor saves (size, opacity, sens, lefty, float, sprint, labels, hints) |
| `touch_buttons` | which of the standard touch buttons this game uses, e.g. `["jump", "menu"]` — anything not listed is removed from the layout and the editor; the stick and look pad are always there |

Where the packager looks for it: the `--assets` dir, the entry file's dir, and
its parent — first hit wins. JSON with `//` comments is accepted.

## Custom launcher icon (Android)

```bash
vulcan package mygame/src/main.vulcan --platform android --assets mygame/assets --icon myicon.png
```

`--icon` takes any PNG (or Android vector XML). Without it you get the
built-in default icon — the default is always there, never forced.

## Packaging

```bash
vulcan package <game>/src/main.vulcan --platform web3d --assets <game>/assets
# open package-<stem>-web3d/index.html in a WebGL2 browser
```
