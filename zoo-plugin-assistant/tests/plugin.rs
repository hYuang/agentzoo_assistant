use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{json, Value};
use zoo_agent::tool::ToolCtx;
use zoo_plugin_assistant::AssistantPlugin;
use zoo_plugins::{Plugin, PluginCtx, PluginRunControl};
use zoo_shared::message::{ProjectId, SessionId};

#[derive(Default)]
struct FakeRuns {
    created: Mutex<Vec<Value>>,
    started: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl PluginRunControl for FakeRuns {
    async fn create_run(
        &self,
        project_id: &str,
        plugin_id: &str,
        entry_id: &str,
        input: Value,
    ) -> Result<String, String> {
        assert_eq!(project_id, "p1");
        assert_eq!(plugin_id, "assistant");
        assert_eq!(entry_id, "learning");
        self.created.lock().unwrap().push(input);
        Ok("run-1".into())
    }

    async fn start_run(
        &self,
        project_id: &str,
        plugin_id: &str,
        run_id: &str,
        stage: Option<&str>,
    ) -> Result<(), String> {
        assert_eq!(plugin_id, "assistant");
        self.started.lock().unwrap().push((
            project_id.into(),
            run_id.into(),
            stage.unwrap_or_default().into(),
        ));
        Ok(())
    }
}

#[tokio::test]
async fn start_practice_tool_creates_and_starts_a_run() {
    let fake = Arc::new(FakeRuns::default());
    let runs: Arc<dyn PluginRunControl> = fake.clone();

    let dir = std::env::temp_dir().join(format!(
        "zoo-assistant-tool-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let plugin = AssistantPlugin;
    let mut plugin_ctx = PluginCtx::new(&dir);
    plugin_ctx.runs = Some(runs);
    let tool = plugin
        .tools(&plugin_ctx)
        .into_iter()
        .find(|tool| tool.agent == "main")
        .expect("start_practice targets main")
        .tool;

    let ctx = ToolCtx::new(
        &dir,
        SessionId::from("s1"),
        ProjectId::from("p1"),
    );
    let out = tool
        .run(
            &json!({
                "topic": "Decorators",
                "question_count": 5,
                "time_limit_minutes": 15
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(out.contains("run-1"));

    let created = fake.created.lock().unwrap();
    assert_eq!(created[0]["topic"], "Decorators");
    assert_eq!(created[0]["question_count"], 5);
    assert_eq!(created[0]["time_limit_minutes"], 15);
    let started = fake.started.lock().unwrap();
    assert_eq!(
        started[0],
        ("p1".into(), "run-1".into(), "intake".into())
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn workflow_references_are_consistent() {
    let plugin = AssistantPlugin;
    let artifacts = plugin.artifact_types();
    let workflow = plugin.workflows().into_iter().next().expect("workflow");
    assert_eq!(workflow.stages.len(), 5);
    for stage in &workflow.stages {
        for output in &stage.outputs {
            assert!(
                artifacts.iter().any(|artifact| &artifact.id == output),
                "stage {} outputs unknown artifact {output}",
                stage.id
            );
        }
        for dependency in &stage.depends_on {
            assert!(
                workflow.stages.iter().any(|other| &other.id == dependency),
                "stage {} depends on unknown stage {dependency}",
                stage.id
            );
        }
        if let Some(gate) = &stage.gate {
            assert!(
                workflow.gates.iter().any(|other| &other.id == gate),
                "stage {} uses unknown gate {gate}",
                stage.id
            );
        }
    }
    for gate in &workflow.gates {
        assert!(
            workflow.roles.iter().any(|role| role.id == gate.approver),
            "gate {} names unknown approver {}",
            gate.id,
            gate.approver
        );
    }
}
