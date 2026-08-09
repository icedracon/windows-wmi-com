# windows-wmi-com

[![Crates.io](https://img.shields.io/crates/v/windows-wmi-com.svg)](https://crates.io/crates/windows-wmi-com)
[![Docs.rs](https://docs.rs/windows-wmi-com/badge.svg)](https://docs.rs/windows-wmi-com)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

In-process WMI queries via COM for Rust — `CoInitializeEx` + `IWbemLocator::ConnectServer` +
`IWbemServices::ExecQuery` + `IEnumWbemClassObject` drained one row at a time. Aimed at
red-team tooling, endpoint agents, and OPSEC self-checks that need Win32 telemetry
without shelling out to `wmic` or `Get-CimInstance`.

## Status

**`0.1.0-dev`** — pre-alpha, expect breaking changes before `0.1.0`. Part of the
[icedracon Rust offensive AD ecosystem](https://github.com/icedracon).

## What it does

Talks WMI directly through the local COM apartment rather than through the
DCOM-over-RPC path that any cross-process WMI client (including `wmic.exe` and
`Get-CimInstance` from PowerShell) uses. That skips the RPC marshaling round-trip
per row and lands roughly an order of magnitude faster for common local queries.
See MSDN's WMI reference (`IWbemLocator`, `IWbemServices`, `IWbemClassObject`) for
the underlying surface — this crate is a thin, opinionated Rust facade over it.

## Usage

```rust
use windows_wmi_com::Wmi;

let wmi = Wmi::connect_local("ROOT\\CIMV2")?;

for row in wmi.query_raw(
    "SELECT Name, ProcessId, ExecutablePath FROM Win32_Process WHERE Name = 'lsass.exe'",
)? {
    let row = row?;
    let name = row.get("Name").and_then(|v| v.as_str().ok()).unwrap_or("");
    let pid  = row.get("ProcessId").and_then(|v| v.as_u32().ok()).unwrap_or(0);
    println!("{name} pid={pid}");
}
```

For typed rows, implement the `FromWbem` trait yourself — the derive macro is
still on the roadmap.

```rust
use windows_wmi_com::{FromWbem, Row, Result};

struct Process { pid: u32, name: String }

impl FromWbem for Process {
    fn from_wbem(row: &Row) -> Result<Self> {
        Ok(Process {
            pid: row.get("ProcessId").and_then(|v| v.as_u32().ok()).unwrap_or(0),
            name: row.get("Name").and_then(|v| v.as_str().ok()).unwrap_or("").to_owned(),
        })
    }
}
```

## What works / what does not (this version)

- Working: MTA apartment init, locator/services connect against a local namespace,
  `ExecQuery`, enumerator drain, and `VARIANT` → `WmiValue` for the common scalar
  cases plus 1-D `BSTR` arrays.
- Stubbed: only a subset of `VARIANT` types is mapped — anything else lands as
  `WmiValue::Null`. No `FromWbem` derive macro yet.
- Not yet: authenticated remote namespaces, method invocation (`IWbemServices::ExecMethod`),
  event subscriptions (`__InstanceOperationEvent`), `IWbemContext` tuning, async
  enumeration.

Everything above the "working" line is fair game to fail on non-toy queries.

## Related icedracon crates

- [`windows-eventlog-native`](https://github.com/icedracon/windows-eventlog-native) —
  live `EvtQuery`/`EvtSubscribe` wrapper for Security/System/Application channels.
- [`winrm-pentest`](https://github.com/icedracon/winrm-pentest) — async WinRM 2.0
  client (NTLM/Kerberos/CredSSP) for cross-platform remote PowerShell.

Cluster: Windows-native higher-level telemetry + admin surfaces. `windows-wmi-com`
covers the local COM WMI path; the other two cover event logs and remote WSMan.

## License

MIT (c) 2026 [zevs](https://github.com/icedracon)
