//! Reconciliation utilities
//!
//! Helpers for analyzing and validating disk state during reconciliation.

use sigil_core::{
    disk::DiskFormat,
    presig::PresigStatus,
    usage::UsageLog,
};

/// Anomaly types that can be detected during reconciliation
#[derive(Debug, Clone)]
pub enum Anomaly {
    /// Presig marked used but no log entry
    MissingLogEntry { presig_index: u32 },

    /// Log entry exists but presig not marked
    OrphanLogEntry { presig_index: u32 },

    /// Gap in presig indices
    PresigGap { expected: u32, found: u32 },

    /// Timestamps out of order
    TimestampAnomaly { index1: u32, index2: u32 },

    /// Count mismatch
    CountMismatch { header_count: u32, actual_count: u32 },

    /// Invalid signature in log
    InvalidSignature { presig_index: u32 },

    /// Voided presig with log entry (shouldn't happen)
    VoidedWithLog { presig_index: u32 },
}

/// Detailed reconciliation analysis
pub struct ReconciliationAnalysis {
    /// Total presigs on disk
    pub total_presigs: u32,

    /// Presigs marked as used
    pub used_presigs: u32,

    /// Presigs marked as voided
    pub voided_presigs: u32,

    /// Fresh presigs remaining
    pub fresh_presigs: u32,

    /// Log entries count
    pub log_entries: u32,

    /// Detected anomalies
    pub anomalies: Vec<Anomaly>,

    /// Whether analysis passed all checks
    pub passed: bool,
}

/// Analyze a disk for reconciliation
pub fn analyze_disk(disk: &DiskFormat) -> ReconciliationAnalysis {
    let mut anomalies = Vec::new();

    // Count presigs by status
    let mut used_count = 0u32;
    let mut voided_count = 0u32;
    let mut fresh_count = 0u32;

    for presig in &disk.presigs {
        match presig.status {
            PresigStatus::Fresh => fresh_count += 1,
            PresigStatus::Used => used_count += 1,
            PresigStatus::Voided => voided_count += 1,
        }
    }

    let log_count = disk.usage_log.len() as u32;

    // Check header vs actual count
    if disk.header.presig_used != used_count {
        anomalies.push(Anomaly::CountMismatch {
            header_count: disk.header.presig_used,
            actual_count: used_count,
        });
    }

    // Check log count vs used count
    if log_count != used_count {
        // Find specific mismatches
        for (i, presig) in disk.presigs.iter().enumerate() {
            let index = i as u32;
            let has_log = disk.usage_log.find_by_presig_index(index).is_some();

            match presig.status {
                PresigStatus::Used if !has_log => {
                    anomalies.push(Anomaly::MissingLogEntry { presig_index: index });
                }
                PresigStatus::Fresh | PresigStatus::Voided if has_log => {
                    anomalies.push(Anomaly::OrphanLogEntry { presig_index: index });
                }
                PresigStatus::Voided if has_log => {
                    anomalies.push(Anomaly::VoidedWithLog { presig_index: index });
                }
                _ => {}
            }
        }
    }

    // Check for gaps in usage log indices
    let mut last_index: Option<u32> = None;
    for entry in &disk.usage_log.entries {
        if let Some(prev) = last_index {
            // Allow some flexibility (not strictly sequential)
            if entry.presig_index < prev {
                anomalies.push(Anomaly::PresigGap {
                    expected: prev + 1,
                    found: entry.presig_index,
                });
            }
        }
        last_index = Some(entry.presig_index);
    }

    // Check timestamp ordering
    let mut last_timestamp: Option<u64> = None;
    for entry in &disk.usage_log.entries {
        if let Some(prev_ts) = last_timestamp {
            // Allow up to 1 hour backward (clock skew)
            if entry.timestamp + 3600 < prev_ts {
                anomalies.push(Anomaly::TimestampAnomaly {
                    index1: entry.presig_index,
                    index2: entry.presig_index,
                });
            }
        }
        last_timestamp = Some(entry.timestamp);
    }

    ReconciliationAnalysis {
        total_presigs: disk.header.presig_total,
        used_presigs: used_count,
        voided_presigs: voided_count,
        fresh_presigs: fresh_count,
        log_entries: log_count,
        anomalies: anomalies.clone(),
        passed: anomalies.is_empty(),
    }
}

/// Generate a human-readable reconciliation report
pub fn generate_report(analysis: &ReconciliationAnalysis) -> String {
    let mut report = String::new();

    report.push_str("=== Reconciliation Report ===\n\n");

    report.push_str("Presig Status:\n");
    report.push_str(&format!("  Total:  {}\n", analysis.total_presigs));
    report.push_str(&format!("  Used:   {}\n", analysis.used_presigs));
    report.push_str(&format!("  Fresh:  {}\n", analysis.fresh_presigs));
    report.push_str(&format!("  Voided: {}\n", analysis.voided_presigs));
    report.push_str(&format!("\nLog Entries: {}\n", analysis.log_entries));

    if analysis.passed {
        report.push_str("\n✓ All checks passed\n");
    } else {
        report.push_str(&format!("\n✗ {} anomalies detected:\n", analysis.anomalies.len()));
        for (i, anomaly) in analysis.anomalies.iter().enumerate() {
            report.push_str(&format!("  {}. {:?}\n", i + 1, anomaly));
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_core::{
        disk::DiskHeader,
        crypto::DerivationPath,
        presig::PresigColdShare,
        ChildId, PublicKey,
    };

    fn create_test_disk(presig_count: u32, used_count: u32) -> DiskFormat {
        let header = DiskHeader::new(
            ChildId::new([1u8; 32]),
            PublicKey::new([2u8; 33]),
            DerivationPath::ethereum_hardened(0),
            presig_count,
            1700000000,
        );

        let mut presigs: Vec<PresigColdShare> = (0..presig_count)
            .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]))
            .collect();

        // Mark some as used
        for i in 0..used_count as usize {
            presigs[i].mark_used();
        }

        let mut disk = DiskFormat::new(header, presigs);
        disk.header.presig_used = used_count;

        disk
    }

    #[test]
    fn test_clean_disk_analysis() {
        let disk = create_test_disk(100, 0);
        let analysis = analyze_disk(&disk);

        assert!(analysis.passed);
        assert_eq!(analysis.fresh_presigs, 100);
        assert_eq!(analysis.used_presigs, 0);
    }

    #[test]
    fn test_count_mismatch() {
        let mut disk = create_test_disk(100, 10);
        disk.header.presig_used = 5; // Mismatch

        let analysis = analyze_disk(&disk);

        assert!(!analysis.passed);
        assert!(analysis.anomalies.iter().any(|a| matches!(a, Anomaly::CountMismatch { .. })));
    }
}
