use opal_abi::msg::{
    request::{LoadIcon, PreloadIcon, Request},
    response::ResponseError,
};

pub use opal_abi::defs::IconID;

use crate::{send_request_and_get, shm::SharedObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconError {
    /// The icon ID is unknown or invalid.
    UnknownIconID,
    /// The given SharedObject does not have enough space to store the icon.
    NotEnoughSpace,
    /// The icon data is not in a valid format.
    InvalidIconFormat,
    /// The icon dimensions are invalid, currently only 256*256 max is allowed.
    InvalidDimensions,
}

/// Same as [`preload_icon`] but doesn't take the item bytes and assumes it is already available in the given object.
pub fn preload_icon_from(shm_object: &mut SharedObject, size: usize) -> Result<IconID, IconError> {
    let resp = send_request_and_get!(
        Request::PreloadIcon(PreloadIcon::new(shm_object.shm_key(), size)),
        IconPreloaded(i)
    )
    .map_err(|e| match e {
        ResponseError::InvalidDataFormat => IconError::InvalidIconFormat,
        ResponseError::InvalidDimensions => IconError::InvalidDimensions,
        e => panic!("Unexpected error: {:?}", e),
    });
    resp.map(|r| r.id())
}

/// Requests the WM to preload an icon returning it's ID
/// should be in BMP format
///
/// Loads the icon into the given [`SharedObject`] first.
pub fn preload_icon(shm_object: &mut SharedObject, icon: &[u8]) -> Result<IconID, IconError> {
    let shm_data = shm_object.data_mut();
    if shm_data.len() < icon.len() {
        return Err(IconError::NotEnoughSpace);
    }

    shm_data[..icon.len()].copy_from_slice(icon);
    preload_icon_from(shm_object, icon.len())
}

/// Loads the icon into the given [`SharedObject`], returning a slice of the icon data from within the [`SharedObject`].
pub fn load_icon(load_into: &mut SharedObject, id: IconID) -> Result<&mut [u8], IconError> {
    send_request_and_get!(
        Request::LoadIcon(LoadIcon::new(load_into.shm_key()), id),
        LoadedIcon(data)
    )
    .map_err(|e| match e {
        ResponseError::UnknownIcon => IconError::UnknownIconID,
        ResponseError::SharedObjectTooSmall => IconError::NotEnoughSpace,
        e => panic!("Unexpected error: {:?}", e),
    })
    .map(|d| &mut load_into.data_mut()[..d.size_bytes()])
}
