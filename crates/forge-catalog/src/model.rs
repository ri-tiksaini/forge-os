#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Apt,
    Flatpak,
    Mise,
    Oci,
    Stack,
}

impl Backend {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "apt" => Ok(Self::Apt),
            "flatpak" => Ok(Self::Flatpak),
            "mise" => Ok(Self::Mise),
            "oci" => Ok(Self::Oci),
            "stack" => Ok(Self::Stack),
            other => Err(format!("unknown backend: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Flatpak => "flatpak",
            Self::Mise => "mise",
            Self::Oci => "oci",
            Self::Stack => "stack",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Resident,
    OnDemand,
    Manual,
}

impl Activation {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "resident" => Ok(Self::Resident),
            "on-demand" => Ok(Self::OnDemand),
            "manual" => Ok(Self::Manual),
            other => Err(format!("unknown activation: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Resident => "resident",
            Self::OnDemand => "on-demand",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkingMode {
    None,
    NativeSocket,
    SocketProxy,
    Path,
}

impl ParkingMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "none" => Ok(Self::None),
            "native-socket" => Ok(Self::NativeSocket),
            "socket-proxy" => Ok(Self::SocketProxy),
            "path" => Ok(Self::Path),
            other => Err(format!("unknown parking_mode: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub backend: Backend,
    pub packages: Vec<String>,
    pub activation: Activation,
    pub parking_mode: ParkingMode,
    pub autostart: bool,
    pub disk_mb: u32,
    pub ram_idle_mb: u32,
    pub ram_active_mb: u32,
    pub tier_min: u8,
    pub license: String,
    pub popularity: u16, // 0..=100 integer
    pub depends: Vec<String>,
    pub conflicts: Vec<String>,
    pub gui_entry: bool,
    pub cli_entry: Vec<String>,
    pub beginner_note: String,
    pub heavy_warning: String,
}
