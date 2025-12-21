#![no_main]
use libfuzzer_sys::fuzz_target;
use northroot_canonical::{ProfileId, PrincipalId, Timestamp, ToolName, Quantity};
use std::str;

fuzz_target!(|data: &[u8]| {
    // Try to parse as UTF-8 string
    let Ok(s) = str::from_utf8(data) else {
        return;
    };

    // Fuzz identifier parsing
    let _ = ProfileId::parse(s);
    let _ = PrincipalId::parse(s);
    let _ = Timestamp::parse(s);
    let _ = ToolName::parse(s);

    // Fuzz quantity parsing
    let _ = Quantity::dec(s, 0);
    let _ = Quantity::int(s);
    let _ = Quantity::rat(s, s);
    let _ = Quantity::f64(s);
});

