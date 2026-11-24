#![forbid(unsafe_code)]

//! Pure, side-effect-free catalog model, validator, and trigram search index (Blueprint 5.1-5.3).

pub mod index;
pub mod load;
pub mod model;

pub use index::{CatalogIndex, SearchHit};
pub use load::{load_dir, parse_entry_toml, validate_catalog};
pub use model::{Activation, Backend, CatalogEntry, ParkingMode};
