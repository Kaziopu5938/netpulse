use std::slice;
use windows::core::GUID;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;

#[derive(Clone, Debug)]
pub struct Adapter {
    pub guid: String,
    pub name: String,
    pub description: String,
    pub kind: &'static str,
    pub up: bool,
}

/// Runs `f` over every row of the IP Helper interface table.
pub fn with_rows<T>(f: impl FnOnce(&[MIB_IF_ROW2]) -> T) -> Option<T> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table) != ERROR_SUCCESS || table.is_null() {
            return None;
        }
        let rows = slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let result = f(rows);
        FreeMibTable(table as *const _);
        Some(result)
    }
}

fn utf16(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

pub fn guid_string(guid: &GUID) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

pub fn alias(row: &MIB_IF_ROW2) -> String {
    utf16(&row.Alias)
}

pub fn description(row: &MIB_IF_ROW2) -> String {
    utf16(&row.Description)
}

pub fn kind(row: &MIB_IF_ROW2) -> &'static str {
    match row.Type {
        6 => "Ethernet",
        71 => "Wi-Fi",
        23 => "PPP",
        243 | 244 => "Mobile",
        _ => "Other",
    }
}

pub fn is_up(row: &MIB_IF_ROW2) -> bool {
    row.OperStatus == IfOperStatusUp
}

/// Physical-ish interfaces: Ethernet, Wi-Fi, PPP and mobile broadband.
///
/// `GetIfTable2` also reports filter drivers (QoS packet scheduler, WFP layers)
/// and WAN miniports as separate rows. Every byte flows through those too, so
/// summing them multiplies the real speed. Windows marks the real NICs with the
/// `HardwareInterface` bit, which is what we filter on.
pub fn is_eligible(row: &MIB_IF_ROW2) -> bool {
    const HARDWARE_INTERFACE: u8 = 0x01;
    if row.InterfaceAndOperStatusFlags._bitfield & HARDWARE_INTERFACE == 0 {
        return false;
    }
    matches!(row.Type, 6 | 71 | 23 | 243 | 244)
}

pub fn list() -> Vec<Adapter> {
    let mut adapters = with_rows(|rows| {
        rows.iter()
            .filter(|row| is_eligible(row))
            .map(|row| Adapter {
                guid: guid_string(&row.InterfaceGuid),
                name: alias(row),
                description: description(row),
                kind: kind(row),
                up: is_up(row),
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    adapters.sort_by(|a, b| b.up.cmp(&a.up).then(a.name.cmp(&b.name)));
    adapters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_drivers_are_not_listed_as_adapters() {
        let all_rows = with_rows(|rows| rows.len()).unwrap_or(0);
        let adapters = list();
        for adapter in &adapters {
            eprintln!(
                "{} [{}] up={} guid={}",
                adapter.name, adapter.kind, adapter.up, adapter.guid
            );
        }
        assert!(adapters.len() <= all_rows);
        // Filter drivers (QoS/WFP) share the NIC's name but are not hardware
        // interfaces, so at most one physical interface per name should remain.
        let mut names: Vec<&str> = adapters.iter().map(|a| a.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), adapters.len());
    }
}
