# projector

A TUI for managing and launching your project scripts.

Built with [ratatui] and [crossterm].

## Features

- Browse, add, edit, and remove project entries from a central `settings.json`
- Filter by name or parent group
- Launch a project script with `Enter` or `Space`
- Toggle a "parent / group" column to show each entry's group
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

## Keybindings (Home mode)

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
| `d`                       | Remove the selected project       |
| `e`                       | Open the selected script in `$EDITOR` |
| `s`                       | Open `settings.json` in `$EDITOR` |
| `f` / `F` / `/`           | Filter by name / parent           |
| `i`                       | Toggle showing the group column   |
| `r` / `R`                 | Reload `settings.json` from disk  |
| `q` / `Esc` / `Ctrl+C`    | Quit                              |

## License

MIT — see [LICENSE](./LICENSE).

[ratatui]: https://ratatui.rs
[crossterm]: https://crossterm.github.io/
