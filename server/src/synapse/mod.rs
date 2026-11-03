use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::Engine;
use rand::thread_rng;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use zoo_server::service::ServerState;
use zoo_server::ApiError;
use zoo_shared::message::ProjectId;

use crate::runs;

pub(crate) mod api;

const DEFAULT_GATEWAY_URL: &str = "https://mcode.cloud";
const DEFAULT_DURATION_MINUTES: u32 = 30;
const DEFAULT_PAPER_TITLE: &str = "AI 同步练习";
const DEFAULT_KNOWLEDGE_TITLE: &str = "AI 知识图谱";
const TOPIC_TITLE_SUFFIX: &str = " · 限时练习";
const DEFAULT_SECTION_NAME: &str = "练习";
const PAPER_TYPE_PRACTICE: &str = "practice";
const QUIZ_ARTIFACT_ID: &str = "quiz";
const KNOWLEDGE_ARTIFACT_ID: &str = "knowledge";
const SYNAPSE_ARTIFACT_ID: &str = "synapse";
const STUDIO_OK_CODE: i64 = 0;
const STUDIO_TOKEN_EXPIRED_CODE: i64 = 5012;
const GATEWAY_PUBLIC_KEY_PATH: &str = "/gateway/public/service/studio/v1/AuthService/GetPublicKey";
const GATEWAY_LOGIN_PATH: &str = "/gateway/public/service/studio/v1/AuthService/Login";
const GATEWAY_REFRESH_PATH: &str = "/gateway/public/service/studio/v1/AuthService/RefreshToken";
const GATEWAY_KNOWLEDGE_CREATE_PATH: &str =
    "/gateway/api/service/studio/v1/QuizService/KnowledgeCreate";
const GATEWAY_KNOWLEDGE_UPDATE_PATH: &str =
    "/gateway/api/service/studio/v1/QuizService/KnowledgeUpdate";
const GATEWAY_QUESTION_CREATE_PATH: &str =
    "/gateway/api/service/studio/v1/QuizService/QuestionCreate";
const GATEWAY_QUESTION_DELETE_PATH: &str =
    "/gateway/api/service/studio/v1/QuizService/QuestionDelete";
const GATEWAY_PAPER_CREATE_PATH: &str = "/gateway/api/service/studio/v1/QuizService/PaperCreate";
const GATEWAY_PAPER_DELETE_PATH: &str = "/gateway/api/service/studio/v1/QuizService/PaperDelete";

#[derive(Clone)]
pub struct SynapseConfig {
    pub gateway_url: String,
    pub tenant_id: String,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
}

impl SynapseConfig {
    pub fn from_env() -> Self {
        Self {
            gateway_url: env("SYNAPSE_GATEWAY_URL")
                .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            tenant_id: env("SYNAPSE_TENANT_ID").unwrap_or_default(),
            token: env("SYNAPSE_STUDIO_TOKEN"),
            refresh_token: env("SYNAPSE_STUDIO_REFRESH_TOKEN"),
        }
    }

    pub fn configured(&self) -> bool {
        !self.tenant_id.is_empty() && (self.token.is_some() || self.refresh_token.is_some())
    }

    pub fn summary(&self) -> Value {
        json!({
            "configured": self.configured(),
            "gateway_url": self.gateway_url,
            "tenant_id": self.tenant_id,
            "auth": if self.refresh_token.is_some() {
                "refresh_token"
            } else if self.token.is_some() {
                "token"
            } else {
                "none"
            },
        })
    }
}

fn env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Serialize, Deserialize)]
struct StudioSession {
    tenant_id: String,
    user_account: String,
    name: String,
    token: String,
    refresh_token: Option<String>,
    #[serde(with = "in_unix_epoch_secs")]
    expires_at: Instant,
}

mod in_unix_epoch_secs {
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use serde::Deserialize;

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let remaining = instant.saturating_duration_since(Instant::now());
        let epoch = elapsed + remaining;
        serializer.serialize_u64(epoch.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Instant::now() + Duration::from_secs(secs))
    }
}

static SESSION: OnceLock<Mutex<Option<StudioSession>>> = OnceLock::new();

fn session_slot() -> &'static Mutex<Option<StudioSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

fn session_snapshot() -> Option<StudioSession> {
    session_slot().lock().ok().and_then(|guard| guard.clone())
}

fn session_summary() -> Option<Value> {
    session_snapshot().map(|session| {
        let expires_in = session
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_secs();
        json!({
            "logged_in": true,
            "tenant_id": session.tenant_id,
            "user_account": session.user_account,
            "name": session.name,
            "expires_in": expires_in,
        })
    })
}

fn store_session(session: StudioSession) {
    if let Ok(mut guard) = session_slot().lock() {
        *guard = Some(session);
    }
}

fn clear_session() {
    if let Ok(mut guard) = session_slot().lock() {
        *guard = None;
    }
}

fn credential_file_path(project_path: &std::path::Path) -> std::path::PathBuf {
    project_path
        .join(".zoo_agent")
        .join("assistant")
        .join("studio-credentials.json")
}

async fn load_credential_file(project_path: &std::path::Path) -> Option<StudioSession> {
    let path = credential_file_path(project_path);
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    serde_json::from_str(&content).ok()
}

async fn save_credential_file(project_path: &std::path::Path, session: &StudioSession) {
    let path = credential_file_path(project_path);
    if let Some(parent) = path.parent() {
        if let Err(error) = tokio::fs::create_dir_all(parent).await {
            tracing::warn!("create studio credential dir failed: {error}");
            return;
        }
    }
    match serde_json::to_string_pretty(session) {
        Ok(content) => {
            if let Err(error) = tokio::fs::write(&path, content).await {
                tracing::warn!("write studio credential file failed: {error}");
            }
        }
        Err(error) => tracing::warn!("serialize studio credential failed: {error}"),
    }
}

async fn clear_credential_file(project_path: &std::path::Path) {
    let path = credential_file_path(project_path);
    if let Err(error) = tokio::fs::remove_file(&path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("remove studio credential file failed: {error}");
        }
    }
}

async fn restore_session(state: &ServerState, project_id: &str) {
    if session_snapshot().is_some() {
        return;
    }
    let Some(project) = state.project(&ProjectId::from(project_id)).await else {
        return;
    };
    let Some(session) = load_credential_file(&project.path).await else {
        return;
    };
    if session.expires_at <= Instant::now() {
        return;
    }
    store_session(session);
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublishMode {
    #[default]
    Overwrite,
    Add,
}

#[derive(Debug, Deserialize, Default)]
pub struct PublishOptions {
    #[serde(default)]
    pub mode: Option<PublishMode>,
    #[serde(default)]
    pub replace: bool,
}

impl PublishOptions {
    fn resolve_mode(&self) -> PublishMode {
        self.mode.unwrap_or(if self.replace {
            PublishMode::Overwrite
        } else {
            PublishMode::Add
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub tenant_id: String,
    pub user_account: String,
    pub password: String,
    #[serde(default)]
    pub remember: bool,
}

#[derive(Debug, Deserialize, Default)]
struct QuizFile {
    #[serde(default)]
    title: String,
    #[serde(default)]
    time_limit_minutes: Option<u32>,
    #[serde(default)]
    questions: Vec<QuizQuestion>,
}

#[derive(Debug, Deserialize, Default)]
struct QuizQuestion {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    answer: Value,
    #[serde(default)]
    explanation: String,
    #[serde(default)]
    difficulty: String,
    #[serde(default)]
    concept: String,
}

#[derive(Debug, Deserialize)]
struct StudioEnvelope {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: Value,
}

struct StudioClient {
    http: reqwest::Client,
    config: SynapseConfig,
    tenant_id: String,
    token: String,
    refresh_token: Option<String>,
    from_session: bool,
    project_path: Option<std::path::PathBuf>,
}

impl StudioClient {
    async fn new(
        config: &SynapseConfig,
        project_path: Option<std::path::PathBuf>,
    ) -> Result<Self, ApiError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| ApiError::Internal(format!("studio http client: {error}")))?;
        if let Some(session) = session_snapshot() {
            return Ok(Self {
                http,
                config: config.clone(),
                tenant_id: session.tenant_id,
                token: session.token,
                refresh_token: session.refresh_token,
                from_session: true,
                project_path,
            });
        }
        let tenant_id = config.tenant_id.clone();
        if tenant_id.is_empty() {
            return Err(ApiError::InvalidRequest(
                "Synapse Studio tenant is not configured".into(),
            ));
        }
        let (token, refresh_token) = if let Some(refresh) = &config.refresh_token {
            let (token, new_refresh, _) =
                refresh_tokens(&http, config, &tenant_id, refresh).await?;
            (token, Some(new_refresh.unwrap_or_else(|| refresh.clone())))
        } else if let Some(token) = &config.token {
            (token.clone(), None)
        } else {
            return Err(ApiError::InvalidRequest(
                "Synapse Studio is not configured; log in or set SYNAPSE_STUDIO_REFRESH_TOKEN"
                    .into(),
            ));
        };
        Ok(Self {
            http,
            config: config.clone(),
            tenant_id,
            token,
            refresh_token,
            from_session: false,
            project_path,
        })
    }

    async fn request(&self, path: &str, data: &Value) -> Result<StudioEnvelope, ApiError> {
        let url = format!("{}{}", self.config.gateway_url, path);
        let body = json!({ "tenant_id": self.tenant_id, "data": data });
        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .header("tenant_id", &self.tenant_id)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                ApiError::Internal(format!("studio request {path} failed: {error}"))
            })?;
        let status = response.status();
        let envelope: StudioEnvelope = response.json().await.map_err(|error| {
            ApiError::Internal(format!("studio response {path} is invalid: {error}"))
        })?;
        if !status.is_success() {
            return Err(ApiError::InvalidRequest(format!(
                "studio {path} returned HTTP {status}"
            )));
        }
        Ok(envelope)
    }

    async fn post(&mut self, path: &str, data: Value) -> Result<Value, ApiError> {
        let mut envelope = self.request(path, &data).await?;
        if envelope.code == STUDIO_TOKEN_EXPIRED_CODE {
            self.refresh().await?;
            envelope = self.request(path, &data).await?;
        }
        if envelope.code != STUDIO_OK_CODE {
            return Err(ApiError::InvalidRequest(format!(
                "studio {path} failed: {} ({})",
                envelope.msg, envelope.code
            )));
        }
        Ok(envelope.data)
    }

    async fn refresh(&mut self) -> Result<(), ApiError> {
        let refresh = self.refresh_token.clone().ok_or_else(|| {
            ApiError::InvalidRequest("Synapse Studio session expired; log in again".into())
        })?;
        let (token, new_refresh, expires_in) =
            refresh_tokens(&self.http, &self.config, &self.tenant_id, &refresh).await?;
        let new_refresh = new_refresh.unwrap_or(refresh);
        self.token = token;
        self.refresh_token = Some(new_refresh.clone());
        if self.from_session {
            if let Some(mut session) = session_snapshot() {
                session.token = self.token.clone();
                session.refresh_token = Some(new_refresh);
                session.expires_at = Instant::now() + Duration::from_secs(expires_in);
                store_session(session.clone());
                if let Some(ref path) = self.project_path {
                    save_credential_file(path, &session).await;
                }
            }
        }
        Ok(())
    }
}

async fn public_key(
    http: &reqwest::Client,
    config: &SynapseConfig,
    tenant: &str,
) -> Result<String, ApiError> {
    let url = format!("{}{}", config.gateway_url, GATEWAY_PUBLIC_KEY_PATH);
    let response = http
        .get(&url)
        .header("tenant_id", tenant)
        .send()
        .await
        .map_err(|error| {
            ApiError::Internal(format!("studio public key request failed: {error}"))
        })?;
    let envelope: StudioEnvelope = response.json().await.map_err(|error| {
        ApiError::Internal(format!("studio public key response is invalid: {error}"))
    })?;
    if envelope.code != STUDIO_OK_CODE {
        return Err(ApiError::InvalidRequest(format!(
            "studio public key failed: {} ({})",
            envelope.msg, envelope.code
        )));
    }
    envelope
        .data
        .get("public_key")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ApiError::Internal("studio public key is missing".into()))
}

fn encrypt_password(pem: &str, password: &str) -> Result<String, ApiError> {
    let key = RsaPublicKey::from_public_key_pem(pem)
        .map_err(|error| ApiError::Internal(format!("studio public key is invalid: {error}")))?;
    let mut rng = thread_rng();
    let encrypted = key
        .encrypt(&mut rng, Oaep::new::<Sha256>(), password.as_bytes())
        .map_err(|error| ApiError::Internal(format!("encrypt studio password failed: {error}")))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

async fn refresh_tokens(
    http: &reqwest::Client,
    config: &SynapseConfig,
    tenant: &str,
    token: &str,
) -> Result<(String, Option<String>, u64), ApiError> {
    let url = format!("{}{}", config.gateway_url, GATEWAY_REFRESH_PATH);
    let body = json!({
        "tenant_id": tenant,
        "data": {
            "tenant_id": tenant,
            "refresh_token": token,
        },
    });
    let response = http
        .post(&url)
        .header("tenant_id", tenant)
        .json(&body)
        .send()
        .await
        .map_err(|error| ApiError::Internal(format!("studio token refresh failed: {error}")))?;
    let envelope: StudioEnvelope = response.json().await.map_err(|error| {
        ApiError::Internal(format!("studio token refresh response is invalid: {error}"))
    })?;
    if envelope.code != STUDIO_OK_CODE {
        return Err(ApiError::InvalidRequest(format!(
            "studio token refresh failed: {} ({})",
            envelope.msg, envelope.code
        )));
    }
    let token = envelope
        .data
        .get("token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ApiError::Internal("studio token refresh returned no token".into()))?;
    let refresh = envelope
        .data
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::to_string);
    let expires_in = envelope
        .data
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    Ok((token, refresh, expires_in))
}

pub async fn login(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
    config: &SynapseConfig,
    request: LoginRequest,
) -> Result<Value, ApiError> {
    let user_account = request.user_account.trim();
    if user_account.is_empty() || request.password.is_empty() {
        return Err(ApiError::InvalidRequest(
            "user_account and password are required".into(),
        ));
    }
    let tenant = if request.tenant_id.trim().is_empty() {
        config.tenant_id.clone()
    } else {
        request.tenant_id.trim().to_string()
    };
    if tenant.is_empty() {
        return Err(ApiError::InvalidRequest(
            "tenant_id is required; set SYNAPSE_TENANT_ID or enter a tenant".into(),
        ));
    }
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| ApiError::Internal(format!("studio http client: {error}")))?;
    let pem = public_key(&http, config, &tenant).await?;
    let encrypted = encrypt_password(&pem, &request.password)?;
    let url = format!("{}{}", config.gateway_url, GATEWAY_LOGIN_PATH);
    let body = json!({
        "tenant_id": tenant,
        "data": {
            "tenant_id": tenant,
            "user_account": user_account,
            "password": encrypted,
        },
    });
    let response = http
        .post(&url)
        .header("tenant_id", &tenant)
        .json(&body)
        .send()
        .await
        .map_err(|error| ApiError::Internal(format!("studio login failed: {error}")))?;
    let envelope: StudioEnvelope = response.json().await.map_err(|error| {
        ApiError::Internal(format!("studio login response is invalid: {error}"))
    })?;
    if envelope.code != STUDIO_OK_CODE {
        return Err(ApiError::InvalidRequest(format!(
            "studio login failed: {} ({})",
            envelope.msg, envelope.code
        )));
    }
    let token = envelope
        .data
        .get("token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ApiError::Internal("studio login returned no token".into()))?;
    let refresh_token = envelope
        .data
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::to_string);
    let expires_in = envelope
        .data
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    let user = envelope.data.get("user");
    let name = user
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or(user_account)
        .to_string();
    let user_account = user
        .and_then(|value| value.get("user_account"))
        .and_then(Value::as_str)
        .unwrap_or(user_account)
        .to_string();
    let session = StudioSession {
        tenant_id: tenant.clone(),
        user_account: user_account.clone(),
        name,
        token,
        refresh_token,
        expires_at: Instant::now() + Duration::from_secs(expires_in),
    };
    store_session(session.clone());
    if let Some(project) = state.project(&ProjectId::from(project_id)).await {
        save_credential_file(&project.path, &session).await;
    }
    if request.remember {
        let mut record = load_record(state, project_id, run_id)
            .await?
            .unwrap_or_else(|| json!({}));
        if let Some(obj) = record.as_object_mut() {
            obj.insert("remembered_account".into(), json!(user_account));
            obj.insert("remembered_password".into(), json!(request.password));
        }
        save_record(state, project_id, run_id, &record).await?;
    }
    Ok(json!({ "session": session_summary() }))
}

pub async fn logout(state: &ServerState, project_id: &str, run_id: &str) -> Value {
    clear_session();
    if let Some(project) = state.project(&ProjectId::from(project_id)).await {
        clear_credential_file(&project.path).await;
    }
    if let Ok(Some(mut record)) = load_record(state, project_id, run_id).await {
        if let Some(obj) = record.as_object_mut() {
            obj.remove("remembered_account");
            obj.remove("remembered_password");
            if let Err(error) = save_record(state, project_id, run_id, &record).await {
                tracing::warn!("clear remembered studio credentials failed: {error}");
            }
        }
    }
    json!({ "session": Value::Null })
}

pub async fn status(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
) -> Result<Value, ApiError> {
    restore_session(state, project_id).await;
    let config = SynapseConfig::from_env();
    let session = session_summary();
    let record = load_record(state, project_id, run_id).await?;
    let remembered = record
        .as_ref()
        .and_then(|value| value.get("remembered_account"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let remembered_password = record
        .as_ref()
        .and_then(|value| value.get("remembered_password"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(json!({
        "config": config.summary(),
        "session": session,
        "can_publish": config.configured() || session.is_some(),
        "record": record,
        "remembered_account": remembered,
        "remembered_password": remembered_password,
    }))
}

pub async fn publish(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
    options: PublishOptions,
) -> Result<Value, ApiError> {
    let config = SynapseConfig::from_env();
    if !config.configured() && session_snapshot().is_none() {
        return Err(ApiError::InvalidRequest(
            "Synapse Studio is not configured; log in or set SYNAPSE_STUDIO_REFRESH_TOKEN".into(),
        ));
    }
    let previous = load_record(state, project_id, run_id).await?;
    let mode = options.resolve_mode();

    let content = runs::read_artifact(state, project_id, run_id, QUIZ_ARTIFACT_ID).await?;
    let quiz: QuizFile = serde_json::from_str(&content.content)
        .map_err(|error| ApiError::InvalidRequest(format!("quiz.json is invalid: {error}")))?;
    if quiz.questions.is_empty() {
        return Err(ApiError::InvalidRequest(
            "quiz.json has no questions".into(),
        ));
    }
    let knowledge_content = runs::read_artifact(state, project_id, run_id, KNOWLEDGE_ARTIFACT_ID)
        .await
        .map_err(|_| {
            ApiError::InvalidRequest(
                "knowledge-map.md is missing; run the knowledge stage before publishing".into(),
            )
        })?
        .content;
    if knowledge_content.trim().is_empty() {
        return Err(ApiError::InvalidRequest("knowledge-map.md is empty".into()));
    }
    let run = runs::get_run(state, project_id, run_id).await?;
    let project_path = state
        .project(&ProjectId::from(project_id))
        .await
        .map(|project| project.path);

    let title = paper_title(&quiz, &run.input);
    let duration = paper_duration(&quiz, &run.input);
    let mut client = StudioClient::new(&config, project_path).await?;
    if mode == PublishMode::Overwrite {
        if let Some(record) = &previous {
            delete_previous(&mut client, record).await;
        }
    }

    let sections = parse_knowledge_sections(&knowledge_content, &title);
    let mut knowledge_point_ids = Vec::with_capacity(sections.len());
    for (index, section) in sections.iter().enumerate() {
        let id = if mode == PublishMode::Add {
            create_knowledge_point(&mut client, &section.name, &section.content).await?
        } else {
            sync_knowledge_point(
                &mut client,
                previous.as_ref(),
                index,
                &section.name,
                &section.content,
            )
            .await?
        };
        knowledge_point_ids.push(id);
    }
    let concept_ids = concept_index(&sections);
    let question_ids = create_questions(
        &mut client,
        &quiz.questions,
        &knowledge_point_ids,
        &concept_ids,
    )
    .await?;
    let paper_id = create_paper(&mut client, &title, duration, &question_ids).await?;

    let synced_at = chrono::Utc::now().to_rfc3339();
    let question_count = question_ids.len();
    let paper = json!({
        "paper_id": paper_id,
        "title": title,
        "question_count": question_count,
        "question_ids": question_ids,
        "duration_minutes": duration,
        "synced_at": synced_at,
    });
    let mut papers = if mode == PublishMode::Add {
        previous_papers(previous.as_ref())
    } else {
        Vec::new()
    };
    papers.push(paper);

    let mut knowledge_points = previous_knowledge_points(previous.as_ref());
    if mode == PublishMode::Add || knowledge_points.is_empty() {
        for (section, id) in sections.iter().zip(knowledge_point_ids.iter()) {
            knowledge_points.push(json!({
                "id": id,
                "name": section.name,
                "synced_at": synced_at,
            }));
        }
    }

    let record = json!({
        "run_id": run.id,
        "gateway_url": config.gateway_url,
        "tenant_id": client.tenant_id,
        "paper_id": paper_id,
        "title": title,
        "question_count": question_count,
        "question_ids": question_ids,
        "knowledge_point_id": knowledge_point_ids.first().copied(),
        "knowledge_point_ids": knowledge_point_ids,
        "knowledge_point_count": knowledge_points.len(),
        "duration_minutes": duration,
        "synced_at": synced_at,
        "papers": papers,
        "knowledge_points": knowledge_points,
    });
    save_record(state, project_id, run_id, &record).await?;
    Ok(json!({ "record": record, "already_synced": false }))
}

fn paper_title(quiz: &QuizFile, input: &Value) -> String {
    if !quiz.title.trim().is_empty() {
        return quiz.title.trim().to_string();
    }
    let topic = input
        .get("topic")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if topic.is_empty() {
        DEFAULT_PAPER_TITLE.to_string()
    } else {
        format!("{topic}{TOPIC_TITLE_SUFFIX}")
    }
}

fn paper_duration(quiz: &QuizFile, input: &Value) -> u32 {
    quiz.time_limit_minutes
        .or_else(|| {
            input
                .get("time_limit_minutes")
                .and_then(Value::as_u64)
                .map(|minutes| minutes as u32)
        })
        .unwrap_or(DEFAULT_DURATION_MINUTES)
        .clamp(1, 600)
}

fn markdown_title(content: &str, fallback: &str) -> String {
    for line in content.lines() {
        if let Some(title) = line.trim().strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    if fallback.trim().is_empty() {
        DEFAULT_KNOWLEDGE_TITLE.to_string()
    } else {
        fallback.trim().to_string()
    }
}

async fn sync_knowledge_point(
    client: &mut StudioClient,
    previous: Option<&Value>,
    index: usize,
    name: &str,
    content: &str,
) -> Result<i64, ApiError> {
    if let Some(id) = previous
        .and_then(|record| record.get("knowledge_point_ids"))
        .and_then(Value::as_array)
        .and_then(|ids| ids.get(index))
        .and_then(Value::as_i64)
    {
        let updated = client
            .post(
                GATEWAY_KNOWLEDGE_UPDATE_PATH,
                json!({ "id": id, "name": name, "content": content }),
            )
            .await;
        if updated.is_ok() {
            return Ok(id);
        }
    }
    create_knowledge_point(client, name, content).await
}

async fn create_knowledge_point(
    client: &mut StudioClient,
    name: &str,
    content: &str,
) -> Result<i64, ApiError> {
    let created = client
        .post(
            GATEWAY_KNOWLEDGE_CREATE_PATH,
            json!({ "name": name, "content": content }),
        )
        .await?;
    created
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::Internal("studio knowledge/create returned no id".into()))
}

async fn create_questions(
    client: &mut StudioClient,
    questions: &[QuizQuestion],
    knowledge_point_ids: &[i64],
    concept_ids: &HashMap<String, usize>,
) -> Result<Vec<i64>, ApiError> {
    let mut question_ids = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let point_index = concept_ids
            .get(question.concept.trim())
            .copied()
            .unwrap_or(0)
            .min(knowledge_point_ids.len().saturating_sub(1));
        let payload = question_payload(question, index, knowledge_point_ids[point_index]);
        let created = client.post(GATEWAY_QUESTION_CREATE_PATH, payload).await?;
        let id = created
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| ApiError::Internal("studio question/create returned no id".into()))?;
        question_ids.push(id);
    }
    Ok(question_ids)
}

async fn create_paper(
    client: &mut StudioClient,
    title: &str,
    duration: u32,
    question_ids: &[i64],
) -> Result<i64, ApiError> {
    let sections = json!([{
        "name": DEFAULT_SECTION_NAME,
        "question_type": "",
        "questions": question_ids
            .iter()
            .map(|id| json!({ "question_id": id, "score": 1 }))
            .collect::<Vec<_>>(),
    }]);
    let paper = client
        .post(
            GATEWAY_PAPER_CREATE_PATH,
            json!({
                "title": title,
                "paper_type": PAPER_TYPE_PRACTICE,
                "duration": duration,
                "sections": sections,
            }),
        )
        .await?;
    paper
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::Internal("studio paper/create returned no id".into()))
}

async fn delete_previous(client: &mut StudioClient, record: &Value) {
    for paper in previous_papers(Some(record)) {
        if let Some(paper_id) = paper.get("paper_id").and_then(Value::as_i64) {
            let _ = client
                .post(GATEWAY_PAPER_DELETE_PATH, json!({ "id": paper_id }))
                .await;
        }
        if let Some(ids) = paper.get("question_ids").and_then(Value::as_array) {
            for id in ids.iter().filter_map(Value::as_i64) {
                let _ = client
                    .post(GATEWAY_QUESTION_DELETE_PATH, json!({ "id": id }))
                    .await;
            }
        }
    }
}

fn previous_papers(record: Option<&Value>) -> Vec<Value> {
    let Some(record) = record else {
        return Vec::new();
    };
    if let Some(papers) = record.get("papers").and_then(Value::as_array) {
        return papers.clone();
    }
    let Some(paper_id) = record.get("paper_id").and_then(Value::as_i64) else {
        return Vec::new();
    };
    vec![json!({
        "paper_id": paper_id,
        "title": record.get("title").cloned().unwrap_or(Value::Null),
        "question_count": record.get("question_count").cloned().unwrap_or(Value::Null),
        "question_ids": record.get("question_ids").cloned().unwrap_or_else(|| json!([])),
        "duration_minutes": record.get("duration_minutes").cloned().unwrap_or(Value::Null),
        "synced_at": record.get("synced_at").cloned().unwrap_or(Value::Null),
    })]
}

fn previous_knowledge_points(record: Option<&Value>) -> Vec<Value> {
    let Some(record) = record else {
        return Vec::new();
    };
    if let Some(points) = record.get("knowledge_points").and_then(Value::as_array) {
        return points.clone();
    }
    let Some(id) = record.get("knowledge_point_id").and_then(Value::as_i64) else {
        return Vec::new();
    };
    vec![json!({
        "id": id,
        "name": record.get("knowledge_name").cloned().unwrap_or(Value::Null),
        "synced_at": record.get("synced_at").cloned().unwrap_or(Value::Null),
    })]
}

struct KnowledgeSection {
    id: String,
    name: String,
    content: String,
}

fn parse_knowledge_sections(content: &str, fallback_title: &str) -> Vec<KnowledgeSection> {
    let mut raw: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut current: Option<(String, String, Vec<String>)> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            if let Some(section) = current.take() {
                raw.push(section);
            }
            let (kp_id, name) = split_kp_heading(heading);
            current = Some((kp_id, name, Vec::new()));
        } else if trimmed.starts_with('#') && !trimmed.starts_with("##") {
            current = None;
        } else if let Some((_, _, body)) = &mut current {
            body.push(line.to_string());
        }
    }
    if let Some(section) = current {
        raw.push(section);
    }
    let mut sections = raw
        .into_iter()
        .map(|(id, name, body)| KnowledgeSection {
            id,
            name,
            content: body.join("\n"),
        })
        .filter(|section| !section.name.is_empty() && !section.content.trim().is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        sections.push(KnowledgeSection {
            id: "kp1".into(),
            name: markdown_title(content, fallback_title),
            content: content.to_string(),
        });
    }
    sections
}

fn split_kp_heading(heading: &str) -> (String, String) {
    let heading = heading.trim();
    if let Some(rest) = heading.strip_prefix("kp") {
        let mut digits = String::new();
        for ch in rest.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }
        if !digits.is_empty() {
            let name = rest[digits.len()..]
                .trim()
                .trim_start_matches(['-', ':', '：', ' '])
                .trim();
            if !name.is_empty() {
                return (format!("kp{digits}"), name.to_string());
            }
        }
    }
    (String::new(), heading.to_string())
}

fn concept_index(sections: &[KnowledgeSection]) -> HashMap<String, usize> {
    let mut index = HashMap::new();
    for (position, section) in sections.iter().enumerate() {
        if !section.id.is_empty() {
            index.insert(section.id.to_ascii_lowercase(), position);
        }
        index.insert(section.name.to_string(), position);
    }
    index
}

fn question_payload(question: &QuizQuestion, index: usize, knowledge_point_id: i64) -> Value {
    let studio_type = match question.kind.as_str() {
        "choice" | "single" => "single",
        "multi" => "multi",
        "judge" => "judge",
        "code" | "essay" => "essay",
        "" if !question.options.is_empty() => "single",
        _ => "short_answer",
    };
    let options = question
        .options
        .iter()
        .enumerate()
        .map(|(idx, text)| json!({ "letter": letter_for(idx), "text": text }))
        .collect::<Vec<_>>();
    let answer = if is_choice_type(studio_type) {
        choice_answer_tokens(&question.answer, &question.options)
    } else {
        answer_text(&question.answer)
    };
    let concept = question.concept.trim();
    let tags: Vec<&str> = if concept.is_empty() {
        vec![]
    } else {
        vec![concept]
    };
    let knowledge_point_ids = vec![knowledge_point_id];
    let options_json = serde_json::to_string(&options).unwrap_or_else(|_| "[]".to_string());
    json!({
        "question_type": studio_type,
        "stem": question.question,
        "options": options_json,
        "answer": answer,
        "analysis": question.explanation,
        "difficulty": difficulty(&question.difficulty),
        "sort": index + 1,
        "tags": tags,
        "knowledge_point_ids": knowledge_point_ids,
    })
}

fn is_choice_type(studio_type: &str) -> bool {
    matches!(
        studio_type,
        "listening" | "single" | "multi" | "indefinite" | "judge" | "cloze" | "reading"
    )
}

fn letter_for(index: usize) -> String {
    let mut n = index as u64;
    let mut letter = String::new();
    loop {
        letter.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    letter
}

fn choice_answer_tokens(answer: &Value, options: &[String]) -> String {
    let tokens = match answer {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| option_letter(item, options))
            .collect::<Vec<_>>(),
        _ => option_letter(answer, options)
            .into_iter()
            .collect::<Vec<_>>(),
    };
    serde_json::to_string(&tokens).unwrap_or_else(|_| "[]".to_string())
}

fn option_letter(value: &Value, options: &[String]) -> Option<String> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| letter_for(value as usize)),
        Value::String(text) => {
            let text = text.trim();
            if let Ok(index) = text.parse::<usize>() {
                return Some(letter_for(index));
            }
            if let Some(index) = options.iter().position(|option| option == text) {
                return Some(letter_for(index));
            }
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(letter), None) if letter.is_ascii_alphabetic() => {
                    Some(letter.to_ascii_uppercase().to_string())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn answer_text(answer: &Value) -> String {
    match answer {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Array(items) => items.iter().map(answer_text).collect::<Vec<_>>().join(", "),
        _ => String::new(),
    }
}

fn difficulty(value: &str) -> i64 {
    match value.trim().to_ascii_lowercase().as_str() {
        "easy" => 1,
        "hard" => 3,
        _ => 2,
    }
}

async fn load_record(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
) -> Result<Option<Value>, ApiError> {
    match runs::read_artifact(state, project_id, run_id, SYNAPSE_ARTIFACT_ID).await {
        Ok(content) => return Ok(serde_json::from_str(&content.content).ok()),
        Err(ApiError::FileNotFound(_)) => {}
        Err(error) => return Err(error),
    }
    let Some(path) = sync_file(state, project_id, run_id).await else {
        return Ok(None);
    };
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(serde_json::from_str(&content).ok()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ApiError::Internal(format!(
            "read studio sync record failed: {error}"
        ))),
    }
}

async fn sync_file(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
) -> Option<std::path::PathBuf> {
    let project = state.project(&ProjectId::from(project_id)).await?;
    Some(runs::run_dir(&project.path, run_id).join("synapse.json"))
}

async fn save_record(
    state: &ServerState,
    project_id: &str,
    run_id: &str,
    record: &Value,
) -> Result<(), ApiError> {
    let content = serde_json::to_string_pretty(record)
        .map_err(|error| ApiError::Internal(format!("serialize studio sync record: {error}")))?;
    let result = runs::write_artifact(
        state,
        project_id,
        run_id,
        SYNAPSE_ARTIFACT_ID,
        &crate::api::WriteArtifact {
            content: content.clone(),
            encoding: Some("utf-8".into()),
        },
    )
    .await;
    match result {
        Ok(_) => return Ok(()),
        Err(ApiError::FileNotFound(_)) => {}
        Err(error) => return Err(error),
    }
    if let Some(path) = sync_file(state, project_id, run_id).await {
        tokio::fs::write(path, content).await.map_err(|error| {
            ApiError::Internal(format!("write studio sync record failed: {error}"))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kp_sections() {
        let content = "# 三角函数\n\n## kp1 正弦函数\n\n考纲要求：掌握图像与性质。\n重点：周期性。\n\n## kp2 余弦函数\n\n考纲要求：掌握图像与性质。\n\n## 总览\n\n前置关系：kp1 -> kp2。\n";
        let sections = parse_knowledge_sections(content, "fallback");
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].id, "kp1");
        assert_eq!(sections[0].name, "正弦函数");
        assert!(sections[0].content.contains("周期性"));
        assert_eq!(sections[1].id, "kp2");
        assert_eq!(sections[1].name, "余弦函数");
        assert_eq!(sections[2].id, "");
        assert_eq!(sections[2].name, "总览");
    }

    #[test]
    fn falls_back_to_single_section_without_headings() {
        let content = "只有正文，没有二级标题。";
        let sections = parse_knowledge_sections(content, "回退标题");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "kp1");
        assert_eq!(sections[0].name, "回退标题");
        assert_eq!(sections[0].content, content);
    }

    #[test]
    fn concept_index_maps_ids_and_names() {
        let sections = vec![
            KnowledgeSection {
                id: "kp1".into(),
                name: "正弦函数".into(),
                content: "x".into(),
            },
            KnowledgeSection {
                id: "kp2".into(),
                name: "余弦函数".into(),
                content: "y".into(),
            },
        ];
        let index = concept_index(&sections);
        assert_eq!(index.get("kp1"), Some(&0));
        assert_eq!(index.get("kp2"), Some(&1));
        assert_eq!(index.get("正弦函数"), Some(&0));
        assert_eq!(index.get("unknown"), None);
    }
}
