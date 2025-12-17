use opal_abi::com::{
    request::{LoadIcon, PreloadIcon, RequestKind},
    response::error::ResponseError,
};

pub use opal_abi::com::request::IconID;

use crate::send_request_and_get;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconError {
    /// The icon ID is unknown or invalid.
    UnknownIconID,
}

/// Requests the WM to preload an icon returning it's ID
/// should be in BMP format
pub fn preload_icon(icon: &[u8]) -> Result<IconID, IconError> {
    let resp = send_request_and_get!(
        RequestKind::PreloadIcon(PreloadIcon::new(icon.len())),
        IconPreloaded(i),
        icon
    )
    .map_err(|e| match e {
        ResponseError::InvalidData
        | ResponseError::InvalidMagic
        | ResponseError::InvalidRequestKind
        | ResponseError::PacketTooShort
        | ResponseError::InvalidUtf8
        | ResponseError::UnknownFatalError
        | ResponseError::UnknownWindow
        | ResponseError::UnknownIcon => panic!("Unexpected error: {:?}", e),
    });
    resp.map(|r| r.id())
}

/// Returns the icon data in bmp format
pub fn get_icon_data_bmp(id: IconID) -> Result<Vec<u8>, IconError> {
    send_request_and_get!(RequestKind::LoadIcon(LoadIcon::new(id)), LoadingIcon(loading), then read loading.size()).map_err(|e| {
        match e {
                ResponseError::UnknownIcon => IconError::UnknownIconID,
                ResponseError::InvalidData
                | ResponseError::InvalidMagic
                | ResponseError::InvalidRequestKind
                | ResponseError::PacketTooShort
                | ResponseError::InvalidUtf8
                | ResponseError::UnknownFatalError
                | ResponseError::UnknownWindow => panic!("Unexpected error: {:?}", e),
            }
    })
}
