## lachesis

a cli-based time tracker that monitors your active screentime. it tracks which window you're focused on, records sessions with timestamps, and stores everything in sqlite.

unlike tools that track all running processes equally, lachesis only tracks the **focused window** - so background spotify doesn't count as screentime.

## features

- **focused window tracking**: only tracks what you're actually looking at, not every background process
- **session-based data**: records timestamped sessions (start/end time, process, window title) instead of flat second counters
- **idle detection**: pauses tracking when you step away (configurable timeout)
- **sqlite storage**: fast queries, handles concurrent access, scales to years of data
- **per-machine sync**: each machine writes its own database file - sync the `data/` folder with syncthing or dropbox, no conflicts
- **tags**: tag processes and filter by tag
- **filtering**: whitelist or blacklist processes with regex support
- **cross-platform**: windows (full support), linux and macos (stubs, contributions welcome)

## architecture

```
~/.config/lachesis/
  config.toml              # settings (check interval, idle timeout, filters)
  .machine_id              # stable machine identifier
  .daemon_pid              # pid of the running daemon
  data/
    HOSTNAME_uuid.db       # sqlite database (one per machine)
```

the daemon (`laches_mon`) checks the focused window every 2 seconds. when focus changes, it ends the previous session and starts a new one. all writes go to sqlite - no full-file rewrites, no JSON serialization on every tick.

## usage

```
laches start                       # start the daemon
laches stop                        # stop the daemon

laches list                        # show tracked process usage
laches list --today                # today only
laches list --date 2025-01-15      # specific date
laches list --tag work             # filter by tag

laches tag firefox --add browser
laches tag firefox --remove browser
laches tag firefox --list

laches mode whitelist              # set filter mode
laches mode blacklist
laches mode default

laches whitelist add firefox
laches whitelist add "^chrome.*" --regex
laches whitelist list
laches whitelist clear

laches blacklist add discord
laches blacklist list

laches autostart on                # run daemon on login
laches autostart off

laches config                      # show configuration

laches data export out.json        # export sessions to json
laches data export out.json --duration 7d
laches data delete --duration 7d
laches data delete --all
laches data reset                  # clear all data
```

## development

```
cargo build              # build laches + laches_mon
cargo test               # run all tests
cargo clippy             # lint
cargo fmt                # format
```

see the [issue tracker](https://github.com/ibra/lachesis/issues) for open work.
