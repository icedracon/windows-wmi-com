use crate::error::{Error, Result};
use crate::row::Row;
use crate::FromWbem;

#[cfg(windows)]
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
#[cfg(windows)]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
#[cfg(windows)]
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
#[cfg(windows)]
use windows::Win32::System::Wmi::{
    IEnumWbemClassObject, IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator,
    WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
};
#[cfg(windows)]
use windows_core::{BSTR, PCWSTR, VARIANT};

/// A live COM WMI session bound to a single namespace on the local host.
///
/// Threading: `connect_local` forces the MTA. The instance is deliberately
/// `!Send` — the underlying COM proxies are apartment-scoped and must be
/// touched from the thread that initialized COM.
///
/// Drop order matters here: Rust drops fields in declaration order, so the
/// COM interfaces (`services`, `_locator`) release *first*, and only then
/// does [`ComGuard`] run `CoUninitialize`. Calling `Release` on a live
/// proxy after `CoUninitialize` is undefined behavior — an easy way to
/// earn an `0xc0000005` in a test binary.
pub struct Wmi {
    #[cfg(windows)]
    services: IWbemServices,
    #[cfg(windows)]
    _locator: IWbemLocator,
    #[cfg(windows)]
    _com: ComGuard,

    // Force !Send / !Sync — COM proxies are per-apartment.
    _not_send: std::marker::PhantomData<*const ()>,
}

/// Owns the `CoInitializeEx` reservation for the lifetime of a [`Wmi`].
/// The `initialized` flag distinguishes "we called `CoInitializeEx` and
/// therefore must uninitialize" from "the thread was already in an
/// incompatible apartment (RPC_E_CHANGED_MODE) and we rode along".
#[cfg(windows)]
struct ComGuard {
    initialized: bool,
}

#[cfg(windows)]
impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(windows)]
impl Wmi {
    /// Connect to `\\.\root\<namespace>` in-process. `namespace` is typically
    /// `"ROOT\\CIMV2"`.
    pub fn connect_local(namespace: &str) -> Result<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            // S_OK (0) and S_FALSE (1) both mean "you initialized COM now".
            // RPC_E_CHANGED_MODE means an STA was set up earlier on this
            // thread by someone else — we accept it and don't own the init.
            let com_initialized = if hr.0 == 0 || hr.0 == 1 {
                true
            } else if hr == RPC_E_CHANGED_MODE {
                false
            } else {
                return Err(Error::CoInit(hr.0));
            };

            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;

            let resource: BSTR = format!("\\\\.\\{}", namespace).into();
            let empty: BSTR = BSTR::new();
            let services: IWbemServices =
                locator.ConnectServer(&resource, &empty, &empty, &empty, 0, &empty, None)?;

            // Per-proxy security — the local WMI service enforces impersonation.
            // Skipping this yields E_ACCESSDENIED on the first ExecQuery.
            CoSetProxyBlanket(
                &services,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )?;

            Ok(Self {
                services,
                _locator: locator,
                _com: ComGuard {
                    initialized: com_initialized,
                },
                _not_send: std::marker::PhantomData,
            })
        }
    }

    /// Run a WQL query and return an iterator of untyped rows.
    pub fn query_raw(&self, wql: &str) -> Result<Box<dyn Iterator<Item = Result<Row>>>> {
        unsafe {
            let language: BSTR = "WQL".into();
            let query: BSTR = wql.into();
            let flags = WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY;
            let enumerator: IEnumWbemClassObject =
                self.services.ExecQuery(&language, &query, flags, None)?;

            Ok(Box::new(QueryIter { enumerator }))
        }
    }

    /// Run a WQL query and map each row through `T: FromWbem`.
    pub fn query<T>(&self, wql: &str) -> Result<Box<dyn Iterator<Item = Result<T>>>>
    where
        T: FromWbem + 'static,
    {
        let raw = self.query_raw(wql)?;
        Ok(Box::new(raw.map(|r| r.and_then(|row| T::from_wbem(&row)))))
    }
}

// Drop is intentionally NOT implemented on `Wmi` — see the doc comment on
// the struct. Field-drop order + `ComGuard`'s Drop sequences uninit correctly.

// ---- non-Windows stubs so the crate at least compiles / documents on other OSes.
// Everything here returns an error immediately — this crate is Windows-only in practice.
#[cfg(not(windows))]
impl Wmi {
    pub fn connect_local(_namespace: &str) -> Result<Self> {
        Err(Error::Hr {
            context: "windows-wmi-com is Windows-only",
            hr: 0,
        })
    }

    pub fn query_raw(&self, _wql: &str) -> Result<Box<dyn Iterator<Item = Result<Row>>>> {
        Err(Error::Hr {
            context: "windows-wmi-com is Windows-only",
            hr: 0,
        })
    }

    pub fn query<T>(&self, _wql: &str) -> Result<Box<dyn Iterator<Item = Result<T>>>>
    where
        T: FromWbem + 'static,
    {
        Err(Error::Hr {
            context: "windows-wmi-com is Windows-only",
            hr: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Enumerator wrapper
// ---------------------------------------------------------------------------

#[cfg(windows)]
struct QueryIter {
    enumerator: IEnumWbemClassObject,
}

#[cfg(windows)]
impl Iterator for QueryIter {
    type Item = Result<Row>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut obj: [Option<IWbemClassObject>; 1] = [None];
            let mut returned: u32 = 0;
            let hr = self
                .enumerator
                .Next(WBEM_INFINITE as i32, &mut obj, &mut returned);
            if hr.is_err() {
                return Some(Err(Error::Hr {
                    context: "IEnumWbemClassObject::Next",
                    hr: hr.0,
                }));
            }
            if returned == 0 {
                return None;
            }
            let object = match obj[0].take() {
                Some(o) => o,
                None => return None,
            };
            Some(row_from_object(&object))
        }
    }
}

#[cfg(windows)]
unsafe fn row_from_object(obj: &IWbemClassObject) -> Result<Row> {
    let mut row = Row::new();

    obj.BeginEnumeration(0)?;
    loop {
        let mut name = BSTR::new();
        let mut val = VARIANT::default();
        let mut vtype: i32 = 0;
        let mut flavor: i32 = 0;
        // windows-rs collapses WBEM_S_NO_MORE_DATA (0x40005, a *success* HRESULT)
        // into Ok(()) — no way to tell it apart from a normal Ok. We detect
        // end-of-enumeration via the returned BSTR: WMI leaves it empty when
        // there are no more properties to yield.
        if let Err(e) = obj.Next(0, &mut name, &mut val, &mut vtype, &mut flavor) {
            let _ = obj.EndEnumeration();
            return Err(Error::Windows(e));
        }
        if name.is_empty() {
            break;
        }

        let key = name.to_string();
        let value = crate::value::convert::from_variant(&val);
        row.insert(key, value);
    }
    obj.EndEnumeration()?;
    Ok(row)
}
