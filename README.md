> !! warning: this program is a work-in-progress and is subject to rapid breaking changes in the master branch. contributions are very welcome.

## lachesis (la·kuh·suhs)

lachesis is a completely cli-based, customizable, automatic time tracking tool designed for tracking and viewing screentime. it automatically tracks your window usage and provides an intuitive command line interface for managing and viewing time
spent on applications.

## features

- **automatic time tracking**: constant running daemon (laches_mon) that keeps track of active windows. (default interval: 5000ms).
- **tags**: tag specific windows and group times together
- **customizable rules**: set up rules for tracking or ignoring specific programs. (!! regex support planned)
- **backup and export**: easily export time tracking data in json or html.
- **cross-platform** : support across windows and linux (macOS planned).
- **idle tracking (!! planned):** ability to automatically tag time as "active" or "idle".

## usage

### starting and stopping

use `laches start` to begin monitoring your time tracking, and `laches stop` to stop it.

```bash
laches start
# started monitoring window usage.

laches stop
# stopped monitoring window usage.
```

### autostart

enable automatic startup when you boot your system with:

```bash
laches autostart yes
# enabled booting on startup.
```

to disable autostart:

```bash
laches autostart no
# stopped booting on startup.
```

### list/watch

see all the applications currently being tracked:

```bash
laches list
```

### filtering

blacklist a specific app:

```bash
laches blacklist add test.exe
# stopped displaying metrics for process "test.exe"
```

or use wildcards for patterns (regex support planned!)

```bash
lachesis whitelist add "^chrome.*" --regex
# stops displaying metrics for processes matching the pattern "^chrome.*".
```

### exporting data

export your time tracking data to a file:

```bash
laches export out.json
# exported past 7 days of time tracking information into "out.json"!
```

future options will include html exports.

### deleting data

delete time-tracking activity for the past 7 days (default) with:

```bash
laches delete --duration=7d
# are you sure you want to delete time tracking activity for the past 7 days? (y/N)
```

or delete all recorded time:

```bash
laches delete all
# are you sure you want to delete time tracking activity for all time? (y/N)
```

## development

- !! contributions are welcome: if you have ideas or improvements, check out the issue tracker and start contributing.
- report issues or request features by making an [issue](https://github.com/ibra/lachesis/issues/new?template=Blank+issue).
