## lachesis (la-kuh-suhs)

lachesis is a cli-based, customizable, automatic time tracking tool for monitoring screentime. it tracks the focused window in the background using a lightweight daemon, records timestamped sessions in sqlite, and provides commands for querying, tagging, filtering, and exporting your data.

## features

- **focused window tracking**: background daemon (`laches_mon`) tracks the active foreground window, not every running process. background apps don't count as screentime.
- **session-based data**: records start/end timestamps, process name, exe path, and window title for every focus change. much richer than flat second counters.
- **idle detection**: automatically pauses tracking after configurable idle timeout (no keyboard/mouse input).
- **tags**: tag processes and group tracked time together.
- **filtering**: whitelist or blacklist specific processes with optional regex matching.
- **time range queries**: view usage by today, week, month, specific date, or arbitrary date range.
- **data export**: export tracked sessions to json, optionally filtered by duration.
- **per-machine sync**: each machine writes its own sqlite database. sync the `data/` directory with syncthing or dropbox with zero conflicts.
- **tui dashboard**: interactive terminal dashboard with today view, timeline, trends, and session list (`laches_tui`).
- **cross-platform**: windows (full support), linux and macos (stubs, contributions welcome).

## usage

### monitoring

```
laches start                       # start the background daemon
laches stop                        # stop it
```

### viewing tracked data

```
laches summary                     # quick daily overview with comparisons

laches list                        # show all tracked process usage
laches list --today                # today only
laches list --week                 # last 7 days
laches list --month                # last 30 days
laches list --date 2025-01-15      # specific date
laches list --range "2025-01-01..2025-01-31"
laches list --tag work             # filter by tag
laches list --sessions             # show individual sessions
laches list --verbose              # extra columns (active days, avg, sessions)
```

### tui dashboard

```
laches_tui                         # launch the interactive terminal dashboard
```

navigate with `1`-`4` or `tab` to switch views, `j`/`k` to scroll, `q` to quit.

### tagging

```
laches tag firefox --add browser
laches tag firefox --remove browser
laches tag firefox --list
```

multiple tags at once (comma-separated):

```
laches tag firefox --add "browser,personal"
```

### filtering

set the mode first, then manage patterns:

```
laches mode whitelist              # only track whitelisted processes
laches mode blacklist              # track everything except blacklisted
laches mode default                # track everything (default)
```

manage patterns:

```
laches whitelist add firefox
laches whitelist add "^chrome.*" --regex
laches whitelist remove firefox
laches whitelist list
laches whitelist clear

laches blacklist add discord
laches blacklist remove discord
laches blacklist list
laches blacklist clear
```

### autostart

```
laches autostart on
laches autostart off
```

### configuration

```
laches config                      # show current config
```

### data management

```
laches data export out.json
laches data export out.json --duration 7d

laches data delete --duration 7d
laches data delete --all
laches data reset
```

## architecture

```
~/.config/lachesis/
  config.toml              # settings (check interval, idle timeout, filters)
  .machine_id              # stable machine identifier
  .daemon_pid              # pid of the running daemon
  data/
    HOSTNAME_uuid.db       # sqlite database (one per machine)
```

the daemon checks the focused window every 2 seconds. when focus changes, it ends the previous session and starts a new one. writes go to sqlite, not full-file rewrites.

## development

contributions are welcome. check the [issue tracker](https://github.com/ibra/lachesis/issues) for open tasks, or open a new issue to report bugs or request features.

```
cargo build              # build laches, laches_mon, and laches_tui
cargo test               # run all tests
cargo clippy             # lint
cargo fmt                # format
```
