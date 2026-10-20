use crate::cache::{CacheKeys, CacheWrapper};
use crate::db::{Database, PrdSkill, Requirement};
use crate::llm::{ClaudeClient, TokenStats};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{debug, info};

/// PRD 服务 - 处理 PRD 生成相关的业务逻辑
pub struct PrdService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

/// PRD 生成所需的上下文（需求 + Skill + LLM 凭据）
struct PrdContext {
    requirement: Requirement,
    skill: Option<PrdSkill>,
    /// API 通道凭据；agent（Claude CLI）通道不需要，缺省不报错
    api_key: Option<String>,
    model: Option<String>,
}

/// PRD 生成方式（设置键 `prd_generation_mode`）
///
/// - `api`：走 Anthropic API（需配置 `claude_api_key`）
/// - `agent`：走本机 Claude CLI 一次性调用（使用 CLI 登录态，无需 API Key）
pub const PRD_MODE_SETTING: &str = "prd_generation_mode";

/// 项目上下文路径设置键（分号分隔的本地文件/目录；工作区需求文件、代码库文档等）
pub const PROJECT_CONTEXT_PATHS_SETTING: &str = "project_context_paths";

const CONTEXT_FILE_CAP: usize = 20 * 1024;
const CONTEXT_TOTAL_CAP: usize = 64 * 1024;
const CONTEXT_MAX_FILES: usize = 10;

/// 按字符安全截断（避免 UTF-8 字节边界 panic）
fn truncate_chars(s: &str, max_bytes: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for c in s.chars() {
        let len = c.len_utf8();
        if used + len > max_bytes {
            break;
        }
        out.push(c);
        used += len;
    }
    out
}

/// 收集**项目上下文**（工作区需求文件、代码库文档等，MA-新需求 #1）。
///
/// 设置 `project_context_paths` 为分号/逗号分隔的本地路径：
/// - 文件：直接读取（.md/.txt/.rst/.json）
/// - 目录：收集其中同扩展名文件（最多 [`CONTEXT_MAX_FILES`] 个）
/// 单文件与总量分别限幅，防止撑爆 prompt。
pub async fn collect_project_context(db: &Arc<Mutex<Database>>) -> Option<String> {
    let paths_raw = {
        let db = db.lock().await;
        db.get_setting(PROJECT_CONTEXT_PATHS_SETTING)
            .ok()
            .flatten()?
    };

    let paths: Vec<&str> = paths_raw
        .split([';', '；', ',', '，'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if paths.is_empty() {
        return None;
    }

    let mut buf = String::new();
    let mut used = 0usize;
    let mut files = 0usize;

    for raw in paths {
        let path = std::path::Path::new(raw);
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        if path.is_file() {
            candidates.push(path.to_path_buf());
        } else if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut list: Vec<std::path::PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|f| {
                        f.is_file()
                            && matches!(
                                f.extension().and_then(|e| e.to_str()),
                                Some("md") | Some("txt") | Some("rst") | Some("json")
                            )
                    })
                    .collect();
                list.sort();
                candidates.extend(list.into_iter().take(CONTEXT_MAX_FILES));
            }
        }

        for file in candidates {
            if files >= CONTEXT_MAX_FILES || used >= CONTEXT_TOTAL_CAP {
                break;
            }
            let Ok(content) = std::fs::read_to_string(&file) else {
                continue;
            };
            if content.trim().is_empty() {
                continue;
            }
            let take = content.len().min(CONTEXT_FILE_CAP).min(CONTEXT_TOTAL_CAP - used);
            if take == 0 {
                break;
            }
            buf.push_str(&format!("\n#### 来源: {}\n", file.display()));
            buf.push_str(&truncate_chars(&content, take));
            buf.push('\n');
            used += take;
            files += 1;
        }

        if files >= CONTEXT_MAX_FILES || used >= CONTEXT_TOTAL_CAP {
            break;
        }
    }

    if buf.trim().is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// 组装"需求描述 + 补充内容 + 项目上下文"的完整输入文本
async fn compose_description(
    db: &Arc<Mutex<Database>>,
    base: Option<&str>,
    extra_content: Option<&str>,
) -> String {
    let mut desc = base.unwrap_or_default().to_string();

    if let Some(extra) = extra_content.map(str::trim).filter(|s| !s.is_empty()) {
        desc.push_str("\n\n## 补充需求内容\n");
        desc.push_str(extra);
    }

    if let Some(context) = collect_project_context(db).await {
        desc.push_str("\n\n## 项目上下文（来自工作区文件，供推断参考）\n");
        desc.push_str(&context);
    }

    desc
}

impl<'a> PrdService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 解析生成 PRD 所需的上下文。
    ///
    /// Skill 解析优先级：显式 `skill_id` → 项目默认 Skill → 内置默认模板（`None`）。
    /// 这里取完整 Skill 记录而非仅模板，便于日志写入真实的 `skill_type`。
    async fn resolve_context(
        &self,
        requirement_id: &str,
        skill_id: Option<&str>,
    ) -> Result<PrdContext, String> {
        let db = self.db.lock().await;

        let requirement = db
            .get_requirement(requirement_id)
            .map_err(|e| {
                tracing::error!("获取需求失败: {}", e);
                format!("获取需求失败: {}", e)
            })?
            .ok_or_else(|| {
                tracing::error!("需求不存在: {}", requirement_id);
                "需求不存在".to_string()
            })?;

        let resolved_id = if let Some(sid) = skill_id {
            Some(sid.to_string())
        } else if !requirement.project_id.is_empty() {
            debug!("查找项目 {} 的默认Skill", requirement.project_id);
            db.list_project_skills(&requirement.project_id)
                .ok()
                .and_then(|ps| ps.into_iter().find(|ps| ps.is_default).map(|ps| ps.skill_id))
        } else {
            None
        };

        let skill = match resolved_id {
            Some(ref sid) => {
                debug!("获取Skill: skill_id={}", sid);
                db.get_prd_skill(sid).map_err(|e| {
                    tracing::error!("获取Skill失败: {}", e);
                    format!("获取Skill失败: {}", e)
                })?
            }
            None => None,
        };

        let api_key = db.get_setting("claude_api_key").ok().flatten();

        // 模型可配置：settings.llm_model（未配置时使用默认模型）
        let model = db.get_setting("llm_model").ok().flatten();

        Ok(PrdContext {
            requirement,
            skill,
            api_key,
            model,
        })
    }

    /// 读取 PRD 生成方式（api / agent，默认 api）
    async fn read_prd_mode(&self) -> String {
        let db = self.db.lock().await;
        match db.get_setting(PRD_MODE_SETTING) {
            Ok(Some(mode)) if mode == "agent" => "agent".to_string(),
            _ => "api".to_string(),
        }
    }

    /// 通过本机 Claude CLI 生成（agent 通道）。
    ///
    /// Agent 选择：设置 `prd_agent_path` 优先，否则取第一个非 `unavailable` 的注册 Agent。
    /// CLI 文本输出不带 usage，Token 记 0（成本无法归因到 CLI 登录态）。
    async fn run_agent_prompt(&self, prompt: &str) -> Result<(String, TokenStats), String> {
        let (agent_path, model, work_dir) = {
            let db = self.db.lock().await;
            let agent_path = db
                .get_setting("prd_agent_path")
                .ok()
                .flatten()
                .filter(|p| !p.trim().is_empty())
                .or_else(|| {
                    db.get_agents()
                        .ok()?
                        .into_iter()
                        .find(|a| a.status != "unavailable")
                        .map(|a| a.binary_path)
                });
            let model = db.get_setting("llm_model").ok().flatten();
            let work_dir = db.get_setting("project_path").ok().flatten();
            (agent_path, model, work_dir)
        };

        let agent_path = agent_path
            .ok_or_else(|| "未检测到可用的 Agent CLI，请先在 Agent 页面执行「刷新检测」或改用 API 接入".to_string())?;

        info!("通过 Agent CLI 生成: agent={}", agent_path);
        let adapter = crate::agent_adapter::adapter_for(&agent_path);
        let inv = adapter
            .build_oneshot(&agent_path, prompt, model.as_deref(), work_dir.as_deref())
            .ok_or_else(|| "该 Agent 不支持一次性调用（仅终端模式）".to_string())?;
        let text = crate::agent_adapter::execute_oneshot(inv).await?;

        Ok((
            text,
            TokenStats {
                input_tokens: 0,
                output_tokens: 0,
                model: model.unwrap_or_else(|| "claude-cli".to_string()),
            },
        ))
    }

    /// 保存成功结果并记账：PRD 正文、Token 使用、Skill 执行日志（success）。
    #[allow(clippy::too_many_arguments)]
    async fn persist_success(
        &self,
        requirement_id: &str,
        ctx: &PrdContext,
        prd_content: &str,
        token_stats: &TokenStats,
        source: &str,
        agent_type: &str,
        duration_ms: i64,
    ) -> Result<(), String> {
        let input_tokens = token_stats.input_tokens as i64;
        let output_tokens = token_stats.output_tokens as i64;

        {
            let db = self.db.lock().await;

            db.update_requirement(requirement_id, None, None, None, None, Some(prd_content))
                .map_err(|e| {
                    tracing::error!("保存PRD失败: {}", e);
                    format!("保存PRD失败: {}", e)
                })?;

            // 依据模型价格表估算成本，避免 total_cost 恒为 0
            let cost = db.estimate_cost(&token_stats.model, input_tokens, output_tokens, 0, 0, 0);

            db.record_token_usage_ext(
                None,
                Some(requirement_id),
                Some(agent_type),
                Some(&token_stats.model),
                input_tokens,
                output_tokens,
                0,
                0,
                0,
                cost,
                source,
            )
            .map_err(|e| {
                tracing::error!("记录Token使用失败: {}", e);
                format!("记录Token使用失败: {}", e)
            })?;

            // 记录 Skill 执行日志（使用 Skill 自身类型，不再硬编码 feature）
            if let Some(skill) = ctx.skill.as_ref() {
                if let Err(e) = db.record_skill_execution_log(
                    None, // task_id
                    None, // session_id
                    Some(requirement_id),
                    &skill.id,
                    &skill.name,
                    &skill.skill_type,
                    "prd_generation",
                    Some(
                        &serde_json::json!({
                            "title": &ctx.requirement.title,
                            "priority": &ctx.requirement.priority,
                        })
                        .to_string(),
                    ),
                    Some(&format!("生成 {} 字符的PRD", prd_content.len())),
                    input_tokens + output_tokens,
                    "success",
                    Some(duration_ms),
                    None,
                ) {
                    tracing::warn!("记录Skill执行日志失败: {}", e);
                }
            }
        }

        // 失效需求相关缓存，避免前端读到旧的 prd_content
        self.cache
            .manager
            .delete(&CacheKeys::requirement(requirement_id))
            .await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        Ok(())
    }

    /// 记录 PRD 生成失败日志（status='failed' + 错误信息 + 耗时）。
    async fn record_failure(&self, requirement_id: &str, ctx: &PrdContext, error: &str, duration_ms: i64) {
        let Some(skill) = ctx.skill.as_ref() else {
            return;
        };

        let db = self.db.lock().await;
        let input_params = serde_json::json!({
            "title": &ctx.requirement.title,
            "priority": &ctx.requirement.priority,
        })
        .to_string();

        if let Err(e) = db.record_skill_execution_log(
            None,
            None,
            Some(requirement_id),
            &skill.id,
            &skill.name,
            &skill.skill_type,
            "prd_generation",
            Some(&input_params),
            Some(&format!("生成失败: {}", error)),
            0,
            "failed",
            Some(duration_ms),
            Some(error),
        ) {
            tracing::warn!("记录Skill失败日志出错: {}", e);
        }
    }

    /// 生成 PRD 文档
    pub async fn generate_prd(
        &self,
        requirement_id: &str,
        skill_id: Option<&str>,
        extra_content: Option<&str>,
    ) -> Result<String, String> {
        info!("开始生成PRD: requirement_id={}", requirement_id);
        let started = std::time::Instant::now();

        let ctx = self.resolve_context(requirement_id, skill_id).await?;
        let template = ctx.skill.as_ref().map(|s| s.template.clone());
        let description =
            compose_description(&self.db, ctx.requirement.description.as_deref(), extra_content)
                .await;

        // 双通道分派：agent（本机 Claude CLI）优先判断，api 为默认
        let (prd_content, token_stats, source, agent_type) =
            if self.read_prd_mode().await == "agent" {
                let prompt = crate::llm::build_prd_prompt(
                    &ctx.requirement.title,
                    &description,
                    &ctx.requirement.priority,
                    template.as_deref(),
                );
                match self.run_agent_prompt(&prompt).await {
                    Ok((content, stats)) => (content, stats, "agent", "claude-cli"),
                    Err(e) => {
                        tracing::error!("生成PRD失败: {}", e);
                        let duration_ms = started.elapsed().as_millis() as i64;
                        self.record_failure(requirement_id, &ctx, &e, duration_ms).await;
                        return Err(format!("生成PRD失败: {}", e));
                    }
                }
            } else {
                info!("调用Claude API生成PRD...");
                let api_key = ctx.api_key.clone().ok_or_else(|| {
                    "请先配置Claude API Key，或在设置中把 PRD 生成方式切换为 Claude CLI".to_string()
                })?;
                let client = ClaudeClient::new(api_key).with_model(ctx.model.as_deref());
                match client
                    .generate_prd(
                        &ctx.requirement.title,
                        &description,
                        &ctx.requirement.priority,
                        template.as_deref(),
                    )
                    .await
                {
                    Ok(v) => (v.0, v.1, "api", "claude-api"),
                    Err(e) => {
                        tracing::error!("生成PRD失败: {}", e);
                        let duration_ms = started.elapsed().as_millis() as i64;
                        self.record_failure(requirement_id, &ctx, &e, duration_ms).await;
                        return Err(format!("生成PRD失败: {}", e));
                    }
                }
            };
        let duration_ms = started.elapsed().as_millis() as i64;

        info!(
            "PRD生成成功（通道 {}），Token使用: input={}, output={}",
            source, token_stats.input_tokens, token_stats.output_tokens
        );

        self.persist_success(requirement_id, &ctx, &prd_content, &token_stats, source, agent_type, duration_ms)
            .await?;

        info!("PRD生成完成: requirement_id={}", requirement_id);
        Ok(prd_content)
    }

    /// 流式生成 PRD（F049）
    ///
    /// 通过 Tauri 事件把增量文本实时推送给前端：
    /// - `prd:chunk`：`{ requirement_id, delta }`
    /// - `prd:done`：`{ requirement_id, content }`
    ///
    /// 与 `generate_prd` 的差异仅在于读取方式：本方法使用真实 SSE 增量读取，
    /// 其余（Skill 解析、落库、记账）完全一致。
    pub async fn generate_prd_streaming(
        &self,
        requirement_id: &str,
        skill_id: Option<&str>,
        extra_content: Option<&str>,
        app: AppHandle,
    ) -> Result<String, String> {
        info!("开始流式生成PRD: requirement_id={}", requirement_id);
        let started = std::time::Instant::now();

        let ctx = self.resolve_context(requirement_id, skill_id).await?;
        let template = ctx.skill.as_ref().map(|s| s.template.clone());
        let description =
            compose_description(&self.db, ctx.requirement.description.as_deref(), extra_content)
                .await;

        // agent 通道：CLI 文本输出无流式能力，一次性生成后整体推送一次
        // （事件协议不变，前端仍按 chunk→done 处理）
        if self.read_prd_mode().await == "agent" {
            let prompt = crate::llm::build_prd_prompt(
                &ctx.requirement.title,
                &description,
                &ctx.requirement.priority,
                template.as_deref(),
            );

            let (prd_content, token_stats) = match self.run_agent_prompt(&prompt).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("流式生成PRD失败: {}", e);
                    let duration_ms = started.elapsed().as_millis() as i64;
                    self.record_failure(requirement_id, &ctx, &e, duration_ms).await;
                    return Err(format!("生成PRD失败: {}", e));
                }
            };

            let _ = app.emit(
                "prd:chunk",
                serde_json::json!({ "requirement_id": requirement_id, "delta": &prd_content }),
            );

            let duration_ms = started.elapsed().as_millis() as i64;
            self.persist_success(requirement_id, &ctx, &prd_content, &token_stats, "agent", "claude-cli", duration_ms)
                .await?;

            let _ = app.emit(
                "prd:done",
                serde_json::json!({
                    "requirement_id": requirement_id,
                    "content": &prd_content,
                }),
            );

            info!("流式PRD生成完成（agent 通道）: requirement_id={}", requirement_id);
            return Ok(prd_content);
        }

        // 增量文本转发任务：把 LLM 片段以事件形式推给前端
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
        let app_handle = app.clone();
        let req_id = requirement_id.to_string();
        let forwarder = tauri::async_runtime::spawn(async move {
            while let Some(delta) = rx.recv().await {
                let _ = app_handle.emit(
                    "prd:chunk",
                    serde_json::json!({ "requirement_id": req_id, "delta": delta }),
                );
            }
        });

        let client = ClaudeClient::new(ctx.api_key.clone().ok_or(
            "请先配置Claude API Key，或在设置中把 PRD 生成方式切换为 Claude CLI",
        )?)
        .with_model(ctx.model.as_deref());
        let result = client
            .generate_prd_streaming(
                &ctx.requirement.title,
                &description,
                &ctx.requirement.priority,
                template.as_deref(),
                tx,
            )
            .await;

        // 发送端由 client 持有，调用返回后即被释放，转发任务随之自然结束
        let _ = forwarder.await;

        let (prd_content, token_stats) = match result {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("流式生成PRD失败: {}", e);
                let duration_ms = started.elapsed().as_millis() as i64;
                self.record_failure(requirement_id, &ctx, &e, duration_ms).await;
                return Err(format!("生成PRD失败: {}", e));
            }
        };

        let duration_ms = started.elapsed().as_millis() as i64;
        self.persist_success(requirement_id, &ctx, &prd_content, &token_stats, "api", "claude-api", duration_ms)
            .await?;

        let _ = app.emit(
            "prd:done",
            serde_json::json!({
                "requirement_id": requirement_id,
                "content": &prd_content,
            }),
        );

        info!("流式PRD生成完成: requirement_id={}", requirement_id);
        Ok(prd_content)
    }
}
