//! Honeycomb device selection and LED report encoding.

use serde::{Deserialize, Serialize};

pub const REPORT_LEN: usize = 64;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceModel {
    #[default]
    Bravo,
    BravoLite,
}

impl DeviceModel {
    pub const fn product_id(self) -> i32 {
        match self {
            Self::Bravo => 0x1901,
            Self::BravoLite => 0x1909,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Bravo => "bravo",
            Self::BravoLite => "bravo_lite",
        }
    }

    pub const fn report_id(self) -> u8 {
        match self {
            Self::Bravo => 0,
            Self::BravoLite => 0x65,
        }
    }

    /// IOKit takes unnumbered Bravo reports without an ID byte. Numbered
    /// Bravo Lite reports retain their ID byte, matching HIDAPI's macOS path.
    pub fn encode(self, banks: [u8; 4]) -> [u8; REPORT_LEN] {
        let mut report = [0; REPORT_LEN];
        match self {
            Self::Bravo => report[..4].copy_from_slice(&banks),
            Self::BravoLite => {
                report[0] = self.report_id();
                // Lite exposes only the six dual-color landing-gear LEDs.
                report[2] = banks[1] & 0x3f;
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_are_distinct_and_default_is_original() {
        assert_eq!(DeviceModel::default(), DeviceModel::Bravo);
        assert_eq!(DeviceModel::Bravo.product_id(), 0x1901);
        assert_eq!(DeviceModel::BravoLite.product_id(), 0x1909);
        assert_eq!(DeviceModel::Bravo.report_id(), 0);
        assert_eq!(DeviceModel::BravoLite.report_id(), 0x65);
    }

    #[test]
    fn original_bravo_packets_are_unchanged() {
        for banks in [[0; 4], [0xff; 4], [0x81, 0x15, 0x40, 0x02]] {
            let mut expected = [0; REPORT_LEN];
            expected[..4].copy_from_slice(&banks);
            assert_eq!(DeviceModel::Bravo.encode(banks), expected);
        }
    }

    #[test]
    fn lite_packets_contain_only_gear_bits() {
        for bits in 0u8..=255 {
            let mut expected = [0; REPORT_LEN];
            expected[0] = 0x65;
            expected[2] = bits & 0x3f;
            assert_eq!(
                DeviceModel::BravoLite.encode([0xff, bits, 0xff, 0xff]),
                expected
            );
        }
    }

    #[test]
    fn lite_standard_gear_states_use_expected_bits() {
        for bits in [0x00, 0x15, 0x2a] {
            let report = DeviceModel::BravoLite.encode([0xff, bits, 0xff, 0xff]);
            assert_eq!(report.len(), REPORT_LEN);
            assert_eq!(report[0], 0x65);
            assert_eq!(report[2], bits);
            assert!(report[1..2].iter().chain(&report[3..]).all(|&b| b == 0));
        }
    }

    #[test]
    fn lite_preserves_each_individual_gear_bit() {
        for bit in 0..6 {
            let bits = 1 << bit;
            let report = DeviceModel::BravoLite.encode([0, bits, 0, 0]);
            assert_eq!(report[2], bits);
        }
    }
}
