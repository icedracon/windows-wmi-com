//! # windows-wmi-com
//!
//! Pre-alpha (0.1.0-dev). In-process COM WMI client: `CoInitializeEx` (MTA) →
//! `IWbemLocator::ConnectServer` → `IWbemServices::ExecQuery` →
//! `IEnumWbemClassObject` drained one row at a time.
//!
//! See `README.md` for status and scope.

mod error;
mod row;
mod value;
mod wmi;

pub use error::{Error, Result};
pub use row::Row;
pub use value::WmiValue;
pub use wmi::Wmi;

/// Rows are extracted from `IWbemClassObject` into a generic [`Row`] map and
/// then handed to `T::from_wbem` for typed queries. Implement this for your
/// own structs; there is no derive macro yet.
pub trait FromWbem: Sized {
    fn from_wbem(row: &Row) -> Result<Self>;
}
