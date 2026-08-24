# projector

A TUI for managing and launching your project scripts.

Built with [ratatui] and [crossterm].

## Features

- Browse, add, edit, and remove project entries from a central `settings.json`
- Filter by name or parent group
- Launch a project script with `Enter` or `Space` — runs in the background
- Success/failure feedback on the footer status line, with a popup on failure
- Toggle a "parent / group" column to show each entry's group
- Re-sort the list by name, parent group, or script file (`z` to cycle)
- Preview the raw script contents in a side panel
- Open `settings.json` or any individual script in your `$EDITOR`

## Build & install

```
cargo build --release
install -D target/release/projector ~/.local/bin/projector
```

## Usage — config format

Entries live in `~/.projects/settings.json`. Each top-level key is the file name of a
shell script in `~/.projects/`.

```json
{
  "my-service.sh": {
    "name": "My Service",
    "parent": "backend",
    "icon": "🐘"
  },
  "deploy.sh": {
    "name": "Deploy"
  }
}
```

| Field    | Type         | Required | Notes                                   |
| -------- | ------------ | -------- | --------------------------------------- |
| `name`   | `string`     | yes      | Display name in the list                |
| `parent` | `string \| null` | no   | Group shown when group view is toggled  |
| `icon`   | `string \| null` | no   | Leading glyph (typically a single emoji or Nerd Font icon) |

If a listed script file does not exist on disk, the entry is shown in red and
cannot be launched until it is created.

### New scripts

Adding a project whose script file does not exist yet creates a template
next to the config:

```bash
#!/bin/bash
script_folder="$(dirname "$(readlink -f "$0")")"
```

with `0750` permissions, ready to edit.

## Launching a project

Launching (`Enter` / `Space`) runs the script in a background thread, so the TUI
stays responsive:

- `launching <name>` — status line, yellow, while the script runs
- `launch succeeded` — green, on exit code `0`
- `launch failed (exit code N)` — red, plus a popup with the script's
  `stderr` (or `stdout` if stderr is empty)

The status line clears itself after a short delay.

## Configuration

| Environment variable     | Default | Description                          |
| ------------------------ | ------- | ------------------------------------ |
| `PROJECTOR_STATUS_TTL_MS`| `5000`  | Status line lifetime, in milliseconds|

## Keybindings

### Home

| Key                       | Action                            |
| ------------------------- | --------------------------------- |
| `j` / `Down`              | Select next                       |
| `k` / `Up`                | Select previous                   |
| `g`                       | Select first                      |
| `G`                       | Select last                       |
| `h` / `Left`              | Unselect                          |
| `l` / `Right`             | Edit the selected project         |
| `Enter` / `Space`         | Launch the selected script        |
| `a` / `n`                 | Add a new project                 |
| `d`                      | Remove the selected project       |
| `e`                       | Open the selected script in `$EDITOR` |
| `s`                      | Open `settings.json` in `$EDITOR` |
| `f` / `F` / `/`           | Filter by name / parent           |
| `i`                      | Toggle showing the group column   |
| `z`                      | Cycle sort order (name → parent → script) |
| `r` / `R`                 | Reload `settings.json` from disk  |
| `q` / `Esc` / `Ctrl+C`    | Quit                              |

### Add / Edit form

| Key            | Action                       |
| -------------- | ---------------------------- |
| `Up` / `Down` / `Tab`  | Move between fields     |
| `Left` / `Right`       | Move the text caret     |
| `Ctrl+V`               | Paste from the clipboard  |
| `Enter`                  | Save                      |
| `Esc`                    | Cancel (and clear the form) |

### Filter

| Key            | Action                       |
| -------------- | ---------------------------- |
| `Left` / `Right` | Move the text caret          |
| `Ctrl+V`         | Paste from the clipboard     |
| `Enter` / `Tab`    | Apply (empty text clears it) |
| `Esc`                | Clear the filter and return to the list |

### Remove confirmation

| Key                  | Action        |
| -------------------- | ------------- |
| `Enter` / `y` / `Y`  | Confirm remove|
| `Esc` / `n` / `N`    | Cancel        |
| `q`                 | Quit the app  |

### Error popup

| Key     | Action                                   |
| ------- | ---------------------------------------- |
| any key (except `Enter` / `Space`) | Dismiss and return to the list (triggers a reload of the config) |
| `q`     | Quit the app                             |

## License

MIT — see [LICENSE](./LICENSE).

[ratatui]: https://ratatui.rs
[crossterm]: https://crossterm.github.io/
