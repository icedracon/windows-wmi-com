use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum WmiValue {
    String(String),
    U32(u32),
    U64(u64),
    Bool(bool),
    Null,
    Array(Vec<WmiValue>),
}

impl WmiValue {
    pub fn kind(&self) -> &'static str {
        match self {
            WmiValue::String(_) => "String",
            WmiValue::U32(_) => "U32",
            WmiValue::U64(_) => "U64",
            WmiValue::Bool(_) => "Bool",
            WmiValue::Null => "Null",
            WmiValue::Array(_) => "Array",
        }
    }

    pub fn as_str(&self) -> Result<&str> {
        match self {
            WmiValue::String(s) => Ok(s.as_str()),
            other => Err(Error::TypeMismatch {
                name: String::new(),
                expected: "String",
                got: other.kind(),
            }),
        }
    }

    pub fn as_u32(&self) -> Result<u32> {
        match self {
            WmiValue::U32(v) => Ok(*v),
            other => Err(Error::TypeMismatch {
                name: String::new(),
                expected: "U32",
                got: other.kind(),
            }),
        }
    }

    pub fn as_u64(&self) -> Result<u64> {
        match self {
            WmiValue::U64(v) => Ok(*v),
            WmiValue::U32(v) => Ok(*v as u64),
            other => Err(Error::TypeMismatch {
                name: String::new(),
                expected: "U64",
                got: other.kind(),
            }),
        }
    }

    pub fn as_bool(&self) -> Result<bool> {
        match self {
            WmiValue::Bool(b) => Ok(*b),
            other => Err(Error::TypeMismatch {
                name: String::new(),
                expected: "Bool",
                got: other.kind(),
            }),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, WmiValue::Null)
    }
}

#[cfg(windows)]
pub(crate) mod convert {
    //! `windows-core` v0.58 exposes `VARIANT` as an opaque `#[repr(transparent)]`
    //! wrapper around the C VARIANT struct, so we shadow it here with an
    //! identical layout and reach in via a reference transmute. The
    //! `size_of` static assert catches any layout drift on version bumps.
    //!
    //! Only tags we can round-trip into `WmiValue` are decoded; everything
    //! else lands as `WmiValue::Null` (see the TODO block at the bottom).

    use super::WmiValue;
    use windows::Win32::System::Variant::{
        VT_BOOL, VT_BSTR, VT_EMPTY, VT_I1, VT_I2, VT_I4, VT_I8, VT_NULL, VT_UI1, VT_UI2, VT_UI4,
        VT_UI8,
    };
    use windows_core::VARIANT;

    #[repr(C)]
    struct RawVariant {
        vt: u16,
        _r1: u16,
        _r2: u16,
        _r3: u16,
        payload: RawPayload,
        // On x64, C VARIANT is 24 bytes total — pad the payload region to match.
        #[cfg(target_pointer_width = "64")]
        _tail: u64,
    }

    #[repr(C)]
    union RawPayload {
        ll: i64,
        ull: u64,
        l: i32,
        ul: u32,
        i: i16,
        ui: u16,
        b: u8,
        bstr: *mut u16,
        var_bool: i16,
    }

    const _: () = assert!(std::mem::size_of::<RawVariant>() == std::mem::size_of::<VARIANT>());

    /// Length of a BSTR is stored as a 4-byte byte-count immediately before
    /// the string pointer. We use the null terminator instead — WMI property
    /// names and typical string properties never contain embedded NULs, so
    /// this is safer than trusting the prefix on an unfamiliar allocator.
    unsafe fn bstr_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    }

    pub unsafe fn from_variant(v: &VARIANT) -> WmiValue {
        let raw: &RawVariant = &*(v as *const VARIANT as *const RawVariant);
        let tag = raw.vt as i32;
        let p = &raw.payload;

        if tag == VT_NULL.0 as i32 || tag == VT_EMPTY.0 as i32 {
            return WmiValue::Null;
        }
        if tag == VT_BSTR.0 as i32 {
            return WmiValue::String(bstr_to_string(p.bstr));
        }
        if tag == VT_BOOL.0 as i32 {
            return WmiValue::Bool(p.var_bool != 0);
        }
        if tag == VT_I4.0 as i32 {
            return WmiValue::U32(p.l as u32);
        }
        if tag == VT_UI4.0 as i32 {
            return WmiValue::U32(p.ul);
        }
        if tag == VT_I2.0 as i32 {
            return WmiValue::U32(p.i as u32);
        }
        if tag == VT_UI2.0 as i32 {
            return WmiValue::U32(p.ui as u32);
        }
        if tag == VT_I1.0 as i32 {
            return WmiValue::U32((p.b as i8) as u32);
        }
        if tag == VT_UI1.0 as i32 {
            return WmiValue::U32(p.b as u32);
        }
        if tag == VT_I8.0 as i32 {
            return WmiValue::U64(p.ll as u64);
        }
        if tag == VT_UI8.0 as i32 {
            return WmiValue::U64(p.ull);
        }

        // TODO(0.2): decode VT_R4/R8, VT_DATE, VT_DECIMAL, and 1-D SAFEARRAYs
        // (VT_ARRAY | VT_BSTR is the common one — string[] properties).
        WmiValue::Null
    }
}
