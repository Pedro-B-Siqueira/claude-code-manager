//! Process table snapshots: memory per session (process tree RSS) and, when Claude Code's session
//! registry is unavailable, `claude` processes with their working directories (like ClaudeGauge).

use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessRow {
    pub pid: u32,
    pub parent_pid: u32,
    pub rss_kb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessTable {
    rows: HashMap<u32, ProcessRow>,
    children: HashMap<u32, Vec<u32>>,
    names: HashMap<u32, String>,
}

impl ProcessTable {
    pub fn capture() -> Self {
        let command = crate::platform::ps_command(crate::platform::CURRENT);
        let output = Command::new(&command.program).args(&command.args).output();
        match output {
            Ok(output) if output.status.success() => Self::parse(&String::from_utf8_lossy(&output.stdout)),
            _ => Self::default(),
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut table = Self::default();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(pid), Some(parent), Some(rss)) = (fields.next(), fields.next(), fields.next()) else { continue };
            let (Ok(pid), Ok(parent_pid), Ok(rss_kb)) = (pid.parse(), parent.parse(), rss.parse()) else { continue };
            let command = fields.collect::<Vec<_>>().join(" ");
            table.rows.insert(pid, ProcessRow { pid, parent_pid, rss_kb });
            table.children.entry(parent_pid).or_default().push(pid);
            table.names.insert(pid, command);
        }
        table
    }

    /// RSS of a process and all its descendants (the shell, `claude`, MCP servers it started…).
    pub fn tree_rss_mb(&self, root: u32) -> Option<f64> {
        self.rows.get(&root)?;
        let mut total_kb = 0u64;
        let mut pending = vec![root];
        while let Some(pid) = pending.pop() {
            total_kb += self.rows.get(&pid).map_or(0, |row| row.rss_kb);
            pending.extend(self.children.get(&pid).into_iter().flatten().copied());
        }
        Some(total_kb as f64 / 1024.0)
    }

    pub fn is_descendant_of(&self, pid: u32, ancestor: u32) -> bool {
        let mut current = pid;
        for _ in 0..64 {
            if current == ancestor {
                return true;
            }
            match self.rows.get(&current) {
                Some(row) if row.parent_pid != current && row.parent_pid > 1 => current = row.parent_pid,
                _ => return false,
            }
        }
        false
    }

    pub fn claude_pids(&self) -> Vec<u32> {
        let mut pids: Vec<u32> = self
            .names
            .iter()
            .filter(|(_, command)| command.rsplit('/').next() == Some("claude"))
            .map(|(pid, _)| *pid)
            .collect();
        pids.sort_unstable();
        pids
    }
}

/// Working directory of a process (read-only): `/proc` on Linux, `lsof` on macOS.
#[cfg(target_os = "linux")]
pub fn process_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok().map(|path| path.to_string_lossy().into_owned())
}

/// Working directory of a process (read-only): `/proc` on Linux, `lsof` on macOS.
#[cfg(not(target_os = "linux"))]
pub fn process_cwd(pid: u32) -> Option<String> {
    let output = Command::new("/usr/sbin/lsof").args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"]).output().ok()?;
    String::from_utf8_lossy(&output.stdout).lines().find_map(|line| line.strip_prefix('n')).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "  1     0   100 /sbin/launchd\n 100     1  2048 /bin/zsh\n 200   100 512000 claude\n 300   200 10240 node\n 400     1  4096 /usr/local/bin/claude\nbroken line\n";

    #[test]
    fn reads_the_working_directory_of_a_process() {
        let expected = std::env::current_dir().unwrap().canonicalize().unwrap();
        let found = process_cwd(std::process::id()).map(|cwd| std::path::PathBuf::from(cwd).canonicalize().unwrap());
        assert_eq!(found, Some(expected));
    }

    #[test]
    fn sums_memory_across_the_process_tree() {
        let table = ProcessTable::parse(SAMPLE);
        let expected = (2048 + 512_000 + 10_240) as f64 / 1024.0;
        assert!((table.tree_rss_mb(100).unwrap() - expected).abs() < 1e-9);
        assert_eq!(table.tree_rss_mb(999), None);
    }

    #[test]
    fn finds_claude_processes_and_ancestry() {
        let table = ProcessTable::parse(SAMPLE);
        assert_eq!(table.claude_pids(), vec![200, 400]);
        assert!(table.is_descendant_of(300, 100));
        assert!(!table.is_descendant_of(400, 100));
    }

    #[test]
    fn captures_the_real_table() {
        let table = ProcessTable::capture();
        assert!(table.tree_rss_mb(std::process::id()).is_some());
    }
}
