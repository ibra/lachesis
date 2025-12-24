> !! warning: this program is a work-in-progress and is subject to rapid breaking changes in the master branch. contributions are very welcome.

## lachesis (la·kuh·suhs)

lachesis is a completely cli-based, customizable, automatic time tracking tool designed for tracking and viewing screentime. it tracks your process usage and provides an intuitive command line interface for managing, filtering, and exporting time spent on applications.

## features

- **automatic time tracking**: background monitor (`laches_mon`) that tracks active processes at a fixed interval.
- **tags**: tag processes and group tracked time together.
- **filtering rules**: whitelist or blacklist specific processes (with optional regex matching).
- **data export**: export tracked data to json.
- **cross-platform**: windows and linux support with the ability to aggregate data across machines if synced.
- **(!! planned) idle tracking**: automatic detection of idle vs active time.

## usage

### starting and stopping monitoring

start and stop the background monitor:

```bash
laches start
# started monitoring process usage.

laches stop
# stopped monitoring process usage.
```

### listing tracked data

list tracked applications:

```bash
laches list
```

filter the list by tag:

```bash
laches list --tag work
```

show only today’s activity:

```bash
laches list --today
```

list data for a specific date:

```bash
laches list --date 2024-01-01
```

include data from all machines:

```bash
laches list --all-machines
```

### tagging processes

add or remove tags from a process:

```bash
laches tag firefox --add browser
laches tag firefox --remove browser
```

list tags for a process:

```bash
laches tag firefox --list
```

### filtering (whitelist / blacklist)

set the global filtering mode:

```bash
laches config mode whitelist
# or
laches config mode blacklist
```

#### whitelist

only track explicitly allowed processes:

```bash
laches config whitelist add firefox
laches config whitelist add "^chrome.*" --regex
```

remove or inspect whitelist entries:

```bash
laches config whitelist remove firefox
laches config whitelist list
laches config whitelist clear
```

#### blacklist

track everything except excluded processes:

```bash
laches config blacklist add discord
```

manage blacklist entries:

```bash
laches config blacklist remove discord
laches config blacklist list
laches config blacklist clear
```

### configuration

show the current configuration:

```bash
laches config show
```

set a custom data storage path:

```bash
laches config set-store-path /path/to/store
```

enable or disable autostart:

```bash
laches config autostart yes
laches config autostart no
```

### exporting data

export tracked data to a file:

```bash
laches data export out.json
```

export only a specific duration:

```bash
laches data export out.json --duration 7d
```

include data from all machines:

```bash
laches data export out.json --all-machines
```

### deleting data

delete data for a given duration:

```bash
laches data delete --duration 7d
```

delete all stored data:

```bash
laches data delete --all
```

reset **all** stored data and state:

```bash
laches data reset
```

## development

contributions are welcome. if you have ideas or improvements, check out the issue tracker and start contributing. you can also report bugs or request features by opening an [issue](https://github.com/ibra/lachesis/issues/new?template=Blank+issue).
