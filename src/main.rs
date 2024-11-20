use fs4::fs_std::FileExt;
use itertools::Itertools;
use libflatpak::{gio::Cancellable, prelude::*, Installation, Ref, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Seek},
};

const ENTRIES_DIR: &str = "/etc/polycrystal/entries";
const STATE_PATH: &str = "/var/lib/polycrystal/state";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
struct FlatpakDefinition {
    id: String,
    remote: String,
    branch: String,
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
    let entries = fs::read_dir(ENTRIES_DIR)?
        .filter_ok(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map_ok(|entry| {
            fs::read_to_string(entry.path())
                .map(|str| serde_json::from_str::<Vec<FlatpakDefinition>>(&str))
        })
        .flatten()
        .flatten_ok()
        .flatten_ok()
        .collect::<Result<Vec<FlatpakDefinition>, std::io::Error>>()?;

    Ok(HashSet::from_iter(entries))
}

fn open_state() -> io::Result<(File, HashSet<FlatpakDefinition>)> {
    let mut state_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(STATE_PATH)?;
    state_file.lock_exclusive()?;

    let mut str = String::new();
    // if the len read is 0, file was just created
    // we don't return empty set if parsing failed
    if state_file.read_to_string(&mut str)? == 0 {
        return Ok((state_file, HashSet::new()));
    }

    Ok((
        state_file,
        HashSet::from_iter(serde_json::from_str::<Vec<FlatpakDefinition>>(&str)?),
    ))
}

fn write_state(state_file: &mut File, set: &HashSet<FlatpakDefinition>) -> io::Result<()> {
    state_file.set_len(0)?;
    state_file.rewind()?;
    serde_json::to_writer(state_file, set)?;
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    let (mut state_file, state) = open_state()?;
    let entries = read_entries()?;

    let to_install: HashSet<_> = entries.difference(&state).cloned().collect();
    let to_remove: HashSet<_> = state.difference(&entries).cloned().collect();

    if to_install.is_empty() && to_remove.is_empty() {
        return Ok(());
    }

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
        transaction.run(Cancellable::NONE)?;
    }

    let new_state: HashSet<_> = state
        .difference(&to_remove)
        .chain(&to_install)
        .cloned()
        .collect();

    write_state(&mut state_file, &new_state)?;

    Ok(())
}
