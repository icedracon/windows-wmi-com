# windows-wmi-com

**STATUS: pre-alpha (0.1.0-dev). Windows-only. Local COM WMI. NOT production-ready.**

In-process WMI queries via COM — `CoInitializeEx` + `IWbemLocator::ConnectServer`
+ `IWbemServices::ExecQuery` + `IEnumWbemClassObject`. Bypasses the DCOM/RPC
overhead that hits any cross-process WMI client; roughly an order of magnitude
faster for local queries.

## Minimal usage

```rust
use windows_wmi_com::Wmi;

let wmi = Wmi::connect_local("ROOT\\CIMV2")?;
for row in wmi.query_raw("SELECT Name, ProcessId FROM Win32_Process WHERE ProcessId = 4")? {
    let row = row?;
    println!("{:?}", row.get("Name"));
}
```

## Scope right now

- Working: apartment init (MTA), locator/services connect, `ExecQuery`,
  enumerator drain, `VARIANT` → `WmiValue` for the common scalar cases and
  1-D `BSTR` arrays.
- Stubbed / partial: only a subset of `VARIANT` types are mapped; anything
  else lands as `WmiValue::Null` with a comment. `FromWbem` derive helpers
  do not exist yet — implement the trait by hand.
- Not yet: authenticated remote namespaces, method invocation, event
  subscriptions, `IWbemContext` tuning, async enumeration.

Everything above the "working" line is fair game to fail on non-toy queries.

## License

MIT.
