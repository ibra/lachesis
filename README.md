## lachesis (la·kuh·suhs)

lachesis is a cli-based, customizable, automatic time tracking tool for monitoring screentime. it tracks your process usage in the background and provides commands for managing, filtering, and exporting time spent on applications.

## features

- **automatic time tracking**: background daemon (`laches_mon`) polls active processes at a configurable interval.
- **tags**: tag processes and group tracked time together.
- **filtering**: whitelist or blacklist specific processes with optional regex matching.
- **data export**: export tracked data to json, optionally filtered by duration or machine.
- **cross-platform**: windows, linux, and macos. process names are normalized across platforms. data can be aggregated across machines if the store file is synced.
- **atomic persistence**: store writes use temp-file-then-rename to prevent data corruption on crash.
- **(planned) idle tracking**: automatic detection of idle vs active time.

## usage

### monitoring

```
laches start             # start the background daemon
laches stop              # stop it
```

### listing tracked data

```
laches list                        # show all tracked processes
laches list --tag work             # filter by tag
laches list --today                # today only
laches list --date 2025-01-15      # specific date
laches list --all-machines         # include all synced machines
```

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
laches config store-path /path     # set custom store location
```

### data management

```
laches data export out.json
laches data export out.json --duration 7d
laches data export out.json --all-machines

laches data delete --duration 7d
laches data delete --all
laches data reset
```

## development

contributions are welcome. check the [issue tracker](https://github.com/ibra/lachesis/issues) for open tasks, or open a new issue to report bugs or request features.

```
cargo build              # build both laches and laches_mon
cargo test               # run all tests
cargo clippy             # lint
cargo fmt                # format
```
