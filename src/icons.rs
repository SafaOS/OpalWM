use std::sync::RwLock;

use opal_abi::defs::IconID;
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

/// Get an icon by its ID.
pub fn get_icon<R>(id: IconID, and_then: impl FnOnce(&[u8]) -> R) -> Result<R, ()> {
    let storage = ICON_STORAGE
        .read()
        .expect("Failed to acquire read lock on the icon storage");
    let key = id.get() as usize - 1;
    let Some(data) = storage.get(key) else {
        panic!("No such icon with id: {id} should have checked before reaching this")
    };
    Ok(and_then(data))
}
