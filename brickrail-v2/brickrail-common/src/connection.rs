use crate::layout_primitives::TrackConnectionID;
use crate::lifecycle::LayoutElement;

/// Marker type for the connection element kind.
#[derive(Clone, Debug)]
pub struct Connection;

/// Layout data for a connection. Currently empty — the TrackConnectionID
/// already encodes all structural info (including portal vs continuous).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ConnectionData;

impl LayoutElement for Connection {
    type ID = TrackConnectionID;
    type Data = ConnectionData;
}
