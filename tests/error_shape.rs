use windows_wmi_com::{Error, Row, WmiValue};

#[test]
fn type_mismatch_reports_names() {
    let mut row = Row::new();
    row.insert("Enabled", WmiValue::U32(1));
    let err = row.get("Enabled").unwrap().as_str().unwrap_err();
    match err {
        Error::TypeMismatch { expected, got, .. } => {
            assert_eq!(expected, "String");
            assert_eq!(got, "U32");
        }
        other => panic!("expected TypeMismatch, got {:?}", other),
    }
}

#[test]
fn missing_field_error_formats() {
    let err = Error::MissingField("Foo".into());
    let msg = format!("{}", err);
    assert!(
        msg.contains("Foo"),
        "message should name the missing field: {}",
        msg
    );
}

#[test]
fn u32_widens_to_u64() {
    let v = WmiValue::U32(42);
    assert_eq!(v.as_u64().unwrap(), 42);
}
