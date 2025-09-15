use std::{
    io::{self, Write},
    sync::RwLock,
};

use opal_abi::com::request::IconID;
use slab::Slab;

static ICON_STORAGE: RwLock<Slab<Vec<u8>>> = RwLock::new(Slab::new());

pub fn add_icon(data: Vec<u8>) -> IconID {
    let mut storage = ICON_STORAGE
        .write()
        .expect("Failed to acquire a write lock on the icon storage");
    let key = storage.insert(data);
    let id = key + 1;

    assert!(
        id <= u16::MAX as usize,
        "Ran out of storage for another icon"
    );
    IconID::new(id as u16).expect("Shall never happen")
}

pub fn load_icon_to(id: IconID, writer: &mut impl Write) -> io::Result<()> {
    let storage = ICON_STORAGE
        .read()
        .expect("Failed to acquire read lock on the icon storage");
    let key = id.get() as usize - 1;
    let Some(data) = storage.get(key) else {
        panic!("No such icon with id: {id} should have checked before reaching this")
    };
    writer.write_all(&data)?;
    Ok(())
}

pub fn icon_size(id: IconID) -> Option<usize> {
    let storage = ICON_STORAGE
        .read()
        .expect("Failed to acquire read lock on the icon storage");
    let key = id.get() as usize - 1;
    storage.get(key).map(|v| v.len())
}
