use opal_abi::com::request::{LoadIcon, PreloadIcon, RequestKind};

use crate::send_request;
pub use opal_abi::com::request::IconID;

/// Requests the WM to preload an icon returning it's ID
/// should be in BMP format
pub fn preload_icon(icon: &[u8]) -> IconID {
    let resp = send_request!(
        RequestKind::PreloadIcon(PreloadIcon::new(icon.len())),
        IconPreloaded(i),
        icon
    );
    resp.id()
}

/// Returns the icon data in bmp format
pub fn get_icon_data_bmp(id: IconID) -> Vec<u8> {
    send_request!(RequestKind::LoadIcon(LoadIcon::new(id)), LoadingIcon(loading), then read loading.size())
}
