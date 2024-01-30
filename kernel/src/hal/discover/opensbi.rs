//! This module contains opensbi hardware discovery details.
//!
//! TODO unstub

pub type DiscoveryProxy = ();
pub static DISCOVERER: DiscoveryProxy = ();

impl super::HALDiscover for DiscoveryProxy {
    fn discover_setup(&self) {
        log!(Warning, "opensbi discovery is currently a stub, using hardcoded constants")
    }
}
