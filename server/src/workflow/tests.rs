use super::WorkflowApp;
use crate::AssistantApp;

#[test]
fn workflow_references_are_consistent() {
    let plugin = AssistantApp;
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "assistant");
    let workflows = plugin.workflows();
    assert_eq!(workflows.len(), 1);
    let artifacts = plugin.artifact_types();
    for stage in &workflows[0].stages {
        for output in &stage.outputs {
            assert!(
                artifacts.iter().any(|artifact| &artifact.id == output),
                "stage {} outputs unknown artifact {output}",
                stage.id
            );
        }
        for dependency in &stage.depends_on {
            assert!(
                workflows[0]
                    .stages
                    .iter()
                    .any(|other| &other.id == dependency),
                "stage {} depends on unknown stage {dependency}",
                stage.id
            );
        }
        if let Some(gate) = &stage.gate {
            assert!(
                workflows[0].gates.iter().any(|other| &other.id == gate),
                "stage {} uses unknown gate {gate}",
                stage.id
            );
        }
    }
    for gate in &workflows[0].gates {
        assert!(
            workflows[0]
                .roles
                .iter()
                .any(|role| role.id == gate.approver),
            "gate {} names unknown approver {}",
            gate.id,
            gate.approver
        );
    }
    assert!(!plugin.entries().is_empty());
}
