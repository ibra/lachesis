> warning ! this program is a work-in-progress and is subject to rapid breaking changes in the master branch. contributions are very welcome.


## lachesis (la·kuh·suhs)
lachesis is a completely cli-based, customizable, automatic time tracking tool designed for tracking and viewing screentime. it automatically tracks your window usage and provides an intuitive command line interface for managing and viewing your time tracking activities. 

## features
- **automatic time tracking**: constant running daemon (laches_mon) that keeps track of active windows. (default: 5000ms).
- **tags**: tag specific windows and group times together
- **customizable rules**: set up rules for tracking or ignoring specific programs. (!! regex support planned)
- **backups**: ability to create backups of your tracking data.
- **export options**: easily export time tracking data in multiple formats (csv, json etc.).
- **cross-platform (!! in progress)** : support across windows and linux (macOS planned).
- **cloud sync (!! in progress)**: expect cloud sync features to be added soon, to keep your data available across all devices. 

## future plans
- **advanced export formats**: exporting for more formats (pdf,html).
- **better visualization**: gui for visualizing time-tracking export data.
- **cloud sync**: bring-your-own-cloud-provider and sync your time tracking data across devices.

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
laches autostart on
# enabled booting on startup.
```
to disable autostart:
```bash
laches autostart off
# stopped booting on startup.
```

### deleting data
delete time-tracking activity for the past 7 days with:
```bash
laches delete
# are you sure you want to delete time tracking activity for the past 7 days? (y/N)
```
or delete all recorded time:
```bash
laches delete all
# are you sure you want to delete time tracking activity for all time? (y/N)
```

### list/watch
see all the applications currently being tracked:
```bash
laches list
```

### blacklist/whitelist
blacklist a specific app:
```bash
laches blacklist test.exe
# stopped time tracking for process "VALORANT.exe"
```

or use wildcards for patterns (regex support planned!)
```bash
laches blacklist *spotify
# detected wildcard.
# stopped time tracking for processes containing "*spotify".

laches whitelist *spotify
# resumed time tracking for processes containing "*spotify".
```

### exporting data
export your time tracking data to a file:
```bash
laches export out.csv
# exported past 7 days of time tracking information into "out.csv"!

laches export out.json --json
# exported past 7 days of time tracking information into "out.json"!
```
future options will include pdf and html exports.

## development
- contributions are welcome! if you have ideas or improvements, check out the issue tracker and start contributing.
- report issues or request features by making an [issue](https://github.com/ibra/lachesis/issues/new?template=Blank+issue).
