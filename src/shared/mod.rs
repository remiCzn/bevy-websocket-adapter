mod event;
mod handle;
mod resources;
mod router;

pub use event::*;
pub use handle::ConnectionHandle;
pub use resources::*;
pub use router::*;

#[macro_export]
macro_rules! impl_message_type {
    ( $type:ty, $name:expr ) => {
        impl bevy_websocket_adapter::shared::MessageType for $type {
            fn message_type() -> &'static str {
                $name
            }
        }
    };
}
