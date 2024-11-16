# polycrystal

Barebones "automatic" Flatpak installer for distribution-default Flatpak packages.

## What it does

1. Scan /etc/polycrystal/entries/ for Flatpak definition files.
2. Compare against the last installed state stored in /var/lib/polycrystal/state. (we don't compare against the "actual state" because the user may have installed or removed packages manually)
3. Install or remove packages as necessary.

An example systemd service file is included to run the installer on boot.

## Definition example

```json
{
  "id": "org.gnome.clocks",
  "remote": "flathub",
  "branch": "stable",
}
```

## TODOs
- [x] Add a systemd service to run the installer on boot (after RPM tranaction?)
- [ ] Lock the state file
- [ ] Look over this code in general
