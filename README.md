# polycrystal

Barebones "automatic" Flatpak installer for distribution-default Flatpak packages.

## What it does

1. Scan /etc/polycrystal/entries/ for Flatpak definition files.
2. Compare against the last installed state stored in /var/lib/polycrystal/state. (we don't compare against the "actual state" because the user may have installed or removed packages manually)
3. Install or remove packages as necessary.

An example systemd service file is included to run the installer on boot.

## Definition example

```json
[
  {
    "id": "org.gnome.clocks",
    "remote": "flathub",
    "branch": "stable",
  }
]
```

Definitions are stored in /etc/polycrystal/entries/ as JSON files. They are JSON arrays of objects, where each object has the following keys:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Yes | The Flatpak application ID |
| remote | string | Yes | The Flatpak remote name (e.g. "flathub") |
| branch | string | Yes | The branch to install (e.g. "stable") |
