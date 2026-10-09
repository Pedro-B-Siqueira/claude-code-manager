//! Installs the synthetic transcripts under a `projects/` tree laid out like Claude Code's.

use std::fs;
use std::path::{Path, PathBuf};

pub const DEMO_SESSION_ID: &str = "11111111-1111-4111-8111-111111111111";
pub const DEMO_PROJECT_DIR: &str = "-tmp-ccm-fixture-demo-app";

const MAIN_FIXTURE: &str = include_str!("../fixtures/transcripts/main-session.jsonl");
const SUBAGENT_FIXTURE: &str = include_str!("../fixtures/transcripts/subagent.jsonl");

pub struct DemoSession {
    pub main_transcript: PathBuf,
    pub subagent_transcript: PathBuf,
}

pub fn main_fixture_lines() -> Vec<&'static str> {
    MAIN_FIXTURE.lines().collect()
}

pub fn install_demo_session(projects_root: &Path) -> DemoSession {
    let project = projects_root.join(DEMO_PROJECT_DIR);
    let subagents = project.join(DEMO_SESSION_ID).join("subagents");
    fs::create_dir_all(&subagents).expect("fixture dirs");
    let main_transcript = project.join(format!("{DEMO_SESSION_ID}.jsonl"));
    let subagent_transcript = subagents.join("agent-a1.jsonl");
    fs::write(&main_transcript, MAIN_FIXTURE).expect("main fixture");
    fs::write(&subagent_transcript, SUBAGENT_FIXTURE).expect("subagent fixture");
    DemoSession { main_transcript, subagent_transcript }
}
