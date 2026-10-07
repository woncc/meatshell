#[path = "impls/latency.rs"]
pub(crate) mod latency;
#[path = "impls/system.rs"]
pub(crate) mod system;
#[path = "struct/system.rs"]
mod system_types;

pub(crate) use latency::{LocalMachine, LocalSnap};
pub(crate) use system_types::{
    LocalGpuInfo, LocalHardwareInfo, NetHist, SystemSampler, SystemSnapshot, TabStatus, TabStatuses,
};
