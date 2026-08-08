//! Compile-time / structural checks. Do not touch COM.

use std::collections::HashMap;
use windows_wmi_com::{FromWbem, Row, WmiValue};

#[test]
fn value_kind_labels_are_stable() {
    assert_eq!(WmiValue::Null.kind(), "Null");
    assert_eq!(WmiValue::Bool(true).kind(), "Bool");
    assert_eq!(WmiValue::U32(1).kind(), "U32");
    assert_eq!(WmiValue::U64(1).kind(), "U64");
    assert_eq!(WmiValue::String("x".into()).kind(), "String");
    assert_eq!(WmiValue::Array(vec![]).kind(), "Array");
}

#[test]
fn row_roundtrip_via_hashmap() {
    let mut fields = HashMap::new();
    fields.insert("Name".to_string(), WmiValue::String("svchost.exe".into()));
    fields.insert("ProcessId".to_string(), WmiValue::U32(4));

    let row = Row { fields };
    assert_eq!(row.len(), 2);
    assert_eq!(
        row.get("Name").and_then(|v| v.as_str().ok()),
        Some("svchost.exe")
    );
    assert_eq!(row.get("ProcessId").and_then(|v| v.as_u32().ok()), Some(4));
    assert!(row.get("Missing").is_none());
}

#[test]
fn from_wbem_can_be_implemented_by_hand() {
    struct Proc {
        name: String,
        pid: u32,
    }

    impl FromWbem for Proc {
        fn from_wbem(row: &Row) -> windows_wmi_com::Result<Self> {
            let name = row
                .get("Name")
                .ok_or_else(|| windows_wmi_com::Error::MissingField("Name".into()))?
                .as_str()?
                .to_string();
            let pid = row
                .get("ProcessId")
                .ok_or_else(|| windows_wmi_com::Error::MissingField("ProcessId".into()))?
                .as_u32()?;
            Ok(Proc { name, pid })
        }
    }

    let mut row = Row::new();
    row.insert("Name", WmiValue::String("System".into()));
    row.insert("ProcessId", WmiValue::U32(4));

    let p = Proc::from_wbem(&row).unwrap();
    assert_eq!(p.name, "System");
    assert_eq!(p.pid, 4);
}
