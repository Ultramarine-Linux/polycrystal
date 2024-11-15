use itertools::Itertools;
use libflatpak::{gio::Cancellable, prelude::*, Installation, Ref, Transaction};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct FlatpakDefinition {
    pub remote: String,
    pub id: String,
    pub branch: String,
}

impl From<&FlatpakDefinition> for libflatpak::Ref {
    fn from(value: &FlatpakDefinition) -> Self {
        libflatpak::Ref::builder()
            .name(&value.id)
            .branch(&value.branch)
            .arch(libflatpak::default_arch().unwrap())
            .build()
    }
}

fn read_entries() -> io::Result<HashSet<FlatpakDefinition>> {
    let entries = fs::read_dir("/etc/polycrystal/entries")?
        .filter_ok(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map_ok(|entry| {
            fs::read_to_string(entry.path())
                .map(|str| serde_json::from_str::<FlatpakDefinition>(&str))
        })
        .flatten()
        .flatten()
        .collect::<Result<Vec<_>, serde_json::Error>>()?;

    Ok(HashSet::from_iter(entries))
}

fn read_state() -> io::Result<HashSet<FlatpakDefinition>> {
    let str = match fs::read_to_string("/var/lib/polycrystal/state") {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(HashSet::new()),
        r => r,
    }?;
    Ok(HashSet::from_iter(serde_json::from_str::<
        Vec<FlatpakDefinition>,
    >(&str)?))
}

fn write_state(set: &HashSet<FlatpakDefinition>) -> io::Result<()> {
    fs::write("/var/lib/polycrystal/state", serde_json::to_string(set)?)?;
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    let entries = read_entries()?;
    let state = read_state()?;

    let to_install: HashSet<_> = entries.difference(&state).cloned().collect();
    let to_remove: HashSet<_> = state.difference(&entries).cloned().collect();
    let new_state: HashSet<_> = state
        .difference(&to_remove)
        .chain(&to_install)
        .cloned()
        .collect();

    let system_install = Installation::new_system(Cancellable::NONE)?;
    let transaction = Transaction::for_installation(&system_install, Cancellable::NONE)?;

    transaction.set_no_interaction(true);

    for flatpak in to_install.iter() {
        match transaction.add_install(
            &flatpak.remote,
            &Ref::from(flatpak).format_ref().unwrap(),
            &[],
        ) {
            Err(e) if e.kind() == Some(libflatpak::Error::AlreadyInstalled) => (),
            r => r?,
        };
    }

    for flatpak in to_remove.iter() {
        match transaction.add_uninstall(&Ref::from(flatpak).format_ref().unwrap()) {
            Err(e) if e.kind() == Some(libflatpak::Error::NotInstalled) => (),
            r => r?,
        };
    }

    if !transaction.is_empty() {
        transaction.run(None::<&Cancellable>)?;
    } else {
        println!("no work to do!")
    }

    write_state(&new_state)?;

    Ok(())
}
