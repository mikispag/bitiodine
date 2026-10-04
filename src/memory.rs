use std::io;

const MIB: u64 = 1024 * 1024;
const HEADROOM: u64 = 128 * MIB;

/// Startup allocation budget for the disk-backed CLI, including traversal overhead.
#[derive(Debug)]
pub struct MemoryBudget {
    pub available: u64,
    pub total: u64,
    pub sqlite: u64,
    pub traversal: u64,
}

impl MemoryBudget {
    pub fn performance_warning(&self) -> Option<&'static str> {
        (self.total < 32 * 1024 * MIB).then_some(
            "Clustering memory budget is below 32 GiB; performance may be poor or clustering may fail due to insufficient memory.",
        )
    }

    pub fn detect() -> io::Result<Self> {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let mut available = system.available_memory();
        if let Some(group) = system.cgroup_limits() {
            available = available.min(group.free_memory);
        }
        #[cfg(target_os = "linux")]
        {
            available = cgroup_available(available);
        }
        Self::from_available(available)
    }

    fn from_available(available: u64) -> io::Result<Self> {
        // Round 80% down to whole bytes without overflowing even for u64::MAX.
        let total = available / 5 * 4 + available % 5 * 4 / 5;
        if total < 512 * MIB {
            return Err(io::Error::other(format!(
                "Insufficient available RAM for resumable clustering: {} MiB (80% budget {} MiB; need at least 512 MiB)",
                available / MIB, total / MIB
            )));
        }
        let sqlite = total / 2;
        Ok(Self {
            available,
            total,
            sqlite,
            traversal: total - sqlite - HEADROOM,
        })
    }
}

#[cfg(target_os = "linux")]
fn cgroup_available(mut available: u64) -> u64 {
    use std::path::Path;
    // sysinfo handles root cgroup limits; also honor nested ancestor limits.
    let groups = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    for line in groups.lines() {
        let mut fields = line.splitn(3, ':');
        let (Some(id), Some(controllers), Some(group)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (root, limit, usage) = if id == "0" && controllers.is_empty() {
            ("/sys/fs/cgroup", "memory.max", "memory.current")
        } else if controllers
            .split(',')
            .any(|controller| controller == "memory")
        {
            (
                "/sys/fs/cgroup/memory",
                "memory.limit_in_bytes",
                "memory.usage_in_bytes",
            )
        } else {
            continue;
        };
        available = cgroup_headroom(Path::new(root), group, limit, usage, available);
    }
    available
}

#[cfg(target_os = "linux")]
fn cgroup_headroom(
    root: &std::path::Path,
    group: &str,
    limit_name: &str,
    usage_name: &str,
    mut available: u64,
) -> u64 {
    use std::path::{Component, Path};
    let mut directory = root.to_path_buf();
    for component in Path::new(group).components() {
        if let Component::Normal(name) = component {
            directory.push(name);
        }
    }
    loop {
        if let (Ok(limit), Ok(current)) = (
            std::fs::read_to_string(directory.join(limit_name)),
            std::fs::read_to_string(directory.join(usage_name)),
        ) {
            if let (Ok(limit), Ok(current)) = (limit.trim().parse(), current.trim().parse()) {
                available = limited_available(available, limit, current);
            }
        }
        if directory == root || !directory.pop() {
            break;
        }
    }
    available
}

#[cfg(any(target_os = "linux", test))]
fn limited_available(available: u64, limit: u64, current: u64) -> u64 {
    available.min(limit.saturating_sub(current))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_uses_eighty_percent_without_a_fixed_ceiling() {
        for available in [
            640 * MIB,
            8 * 1024 * MIB,
            32 * 1024 * MIB,
            64 * 1024 * MIB,
            256 * 1024 * MIB,
            u64::MAX,
        ] {
            let budget = MemoryBudget::from_available(available).unwrap();
            assert_eq!(u128::from(budget.total), u128::from(available) * 4 / 5);
            assert_eq!(budget.sqlite + budget.traversal + HEADROOM, budget.total);
        }
        for remainder in 0..5 {
            let available = 64 * 1024 * MIB + remainder;
            let budget = MemoryBudget::from_available(available).unwrap();
            assert_eq!(u128::from(budget.total), u128::from(available) * 4 / 5);
        }
        assert!(MemoryBudget::from_available(0).is_err());
        assert!(MemoryBudget::from_available(512 * MIB).is_err());
    }

    #[test]
    fn cgroup_headroom_never_exceeds_host_available_or_underflows() {
        assert_eq!(limited_available(8 * MIB, 5 * MIB, 2 * MIB), 3 * MIB);
        assert_eq!(limited_available(MIB, 5 * MIB, 2 * MIB), MIB);
        assert_eq!(limited_available(MIB, MIB, 2 * MIB), 0);
    }

    #[test]
    fn performance_warning_depends_on_budget_not_available_ram() {
        for available in [640 * MIB, 32 * 1024 * MIB, 40 * 1024 * MIB - 1] {
            let budget = MemoryBudget::from_available(available).unwrap();
            let warning = budget.performance_warning().unwrap();
            assert!(warning.contains("performance") && warning.contains("fail"));
            assert_eq!(u128::from(budget.total), u128::from(available) * 4 / 5);
        }
        for available in [40 * 1024 * MIB, 64 * 1024 * MIB, u64::MAX] {
            assert!(MemoryBudget::from_available(available)
                .unwrap()
                .performance_warning()
                .is_none());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn nested_cgroups_honor_parent_limits_and_unlimited_children() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("bitiodine-memory-{}", std::process::id()));
        let child = root.join("parent/child");
        fs::create_dir_all(&child).unwrap();
        for (limit, usage) in [
            ("memory.max", "memory.current"),
            ("memory.limit_in_bytes", "memory.usage_in_bytes"),
        ] {
            fs::write(root.join(limit), "900").unwrap();
            fs::write(root.join(usage), "100").unwrap();
            fs::write(root.join("parent").join(limit), "500").unwrap();
            fs::write(root.join("parent").join(usage), "300").unwrap();
            fs::write(child.join(limit), "max").unwrap();
            fs::write(child.join(usage), "20").unwrap();
            assert_eq!(
                cgroup_headroom(&root, "/parent/child", limit, usage, 1_000),
                200
            );
            fs::write(child.join(limit), "100").unwrap();
            assert_eq!(
                cgroup_headroom(&root, "/parent/child", limit, usage, 1_000),
                80
            );
            assert_eq!(
                cgroup_headroom(&root, "/parent/child", limit, usage, 50),
                50
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
