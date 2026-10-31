//! Plugin catalog exposed by the host.
//!
//! Plugin composition lives in the product binary (see `app::BuiltinPlugins`),
//! not in the framework.

use zoo_shared::api::{
    PluginChatTaskInfo, PluginEntryInfo, PluginGateInfo, PluginInfo, PluginRoleInfo,
    PluginStageInfo, PluginUiPageInfo, PluginWorkflowInfo, PluginsResponse,
};

use zoo_server::service::ServerState;

use crate::sdk::{Plugin, RoleKind};

/// The public catalog of this app.
pub async fn list(_state: &ServerState) -> PluginsResponse {
    let plugins = vec![crate::AssistantPlugin]
        .into_iter()
        .map(|plugin| {
            let manifest = plugin.manifest();
            PluginInfo {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                api_version: manifest.api_version,
                description: manifest.description.clone(),
                capabilities: manifest.capabilities.clone(),
                requires: manifest.requires.clone(),
                entries: plugin
                    .entries()
                    .into_iter()
                    .map(|entry| PluginEntryInfo {
                        id: entry.id,
                        title: entry.title,
                        description: entry.description,
                        input_schema: entry.input_schema,
                        output_schema: entry.output_schema,
                        ui_page: entry.ui_page,
                    })
                    .collect(),
                workflows: plugin
                    .workflows()
                    .into_iter()
                    .map(|workflow| PluginWorkflowInfo {
                        id: workflow.id,
                        name: workflow.name,
                        roles: workflow
                            .roles
                            .into_iter()
                            .map(|role| PluginRoleInfo {
                                id: role.id,
                                name: role.name,
                                kind: match role.kind {
                                    RoleKind::Human => "human".into(),
                                    RoleKind::Agent => "agent".into(),
                                },
                                capabilities: role.capabilities,
                                can_approve: role.can_approve,
                            })
                            .collect(),
                        gates: workflow
                            .gates
                            .into_iter()
                            .map(|gate| PluginGateInfo {
                                id: gate.id,
                                name: gate.name,
                                approver: gate.approver,
                                on_reject: gate.on_reject,
                            })
                            .collect(),
                        stages: workflow
                            .stages
                            .into_iter()
                            .map(|stage| PluginStageInfo {
                                id: stage.id,
                                name: stage.name,
                                role: stage.role,
                                requires: stage.requires,
                                depends_on: stage.depends_on,
                                outputs: stage.outputs,
                                gate: stage.gate,
                                foreach: stage.foreach,
                            })
                            .collect(),
                    })
                    .collect(),
                ui: plugin
                    .ui_pages()
                    .into_iter()
                    .map(|page| PluginUiPageInfo {
                        id: page.id,
                        title: page.title,
                        page: page.page,
                        slot: page.slot,
                    })
                    .collect(),
                chat_tasks: plugin
                    .chat_tasks()
                    .into_iter()
                    .map(|task| PluginChatTaskInfo {
                        id: task.id,
                        label_zh: task.label_zh,
                        label_en: task.label_en,
                        prompt_zh: task.prompt_zh,
                        prompt_en: task.prompt_en,
                    })
                    .collect(),
            }
        })
        .collect();
    PluginsResponse { plugins }
}
