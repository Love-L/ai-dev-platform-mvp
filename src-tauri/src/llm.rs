use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, error, debug};
use tokio::sync::mpsc;

/// 构建"生成 PRD"提示词。
///
/// API 通道（`ClaudeClient::generate_prd`）与 Claude CLI 通道（`prd_service`
/// 的 agent 模式）**共用**，保证两种接入方式产出结构一致。
pub fn build_prd_prompt(
    title: &str,
    description: &str,
    priority: &str,
    skill_template: Option<&str>,
) -> String {
    if let Some(template) = skill_template {
        // 使用 Skill 模板生成
        let filled_template = template
            .replace("{title}", title)
            .replace("{description}", description)
            .replace("{priority}", priority);

        format!(
            r#"你是一个专业的产品经理。请根据以下需求信息和模板，生成一份结构化的PRD文档。

## 需求信息
- 标题: {}
- 描述: {}
- 优先级: {}

## PRD 模板
请按照以下模板结构生成PRD内容：

{}请直接输出PRD内容，不要包含其他说明。"#,
            title, description, priority, filled_template
        )
    } else {
        // 使用默认模板
        format!(
            r#"你是一个专业的产品经理。请根据以下需求描述，生成一份结构化的PRD文档。

## 需求信息
- 标题: {}
- 描述: {}
- 优先级: {}

## 输出要求
请生成包含以下章节的PRD文档（使用Markdown格式）：

### 1. 需求背景与目标
简述需求的业务背景和要达成的目标。

### 2. 用户故事
以"作为[角色]，我想要[功能]，以便[价值]"的格式描述用户故事。

### 3. 功能描述
详细描述功能需求，包括：
- 核心功能点
- 交互流程
- 异常处理

### 4. 验收标准
列出可测试的验收标准。

### 5. 技术要点
概述技术实现的关键点。

### 6. 风险评估
识别潜在风险和应对措施。

请直接输出PRD内容，不要包含其他说明。"#,
            title, description, priority
        )
    }
}

/// 构建"需求智能完善"提示词（输出 JSON 字段契约与 API 通道一致）。
/// LLM 拆解出的候选任务（前端勾选后批量落库）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakdownTask {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_task_type")]
    pub task_type: String,
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_task_type() -> String {
    "development".to_string()
}

fn default_priority() -> String {
    "P2".to_string()
}

/// 消毒 LLM 输出：枚举白名单 + 数量上限，防止脏值直落任务表
pub fn sanitize_breakdown_tasks(tasks: Vec<BreakdownTask>) -> Vec<BreakdownTask> {
    tasks
        .into_iter()
        .filter(|t| !t.title.trim().is_empty())
        .take(20)
        .map(|mut t| {
            t.title = t.title.trim().to_string();
            if !["development", "testing", "bug_fix"].contains(&t.task_type.as_str()) {
                t.task_type = "development".to_string();
            }
            if !["P0", "P1", "P2", "P3"].contains(&t.priority.as_str()) {
                t.priority = "P2".to_string();
            }
            t.description = t.description.trim().to_string();
            t
        })
        .collect()
}

/// 智能拆解 prompt：需求 + PRD → 结构化任务清单（P1-⑪）
pub fn build_breakdown_prompt(title: &str, description: &str, prd: &str) -> String {
    format!(
        r#"你是一个资深技术负责人。请把以下需求拆解为可执行的开发任务清单。

## 需求
- 标题: {}
- 描述: {}

## PRD
{}

## 输出要求
只输出一个 JSON 数组（不要任何其他文字、解释或 markdown 围栏），数组每项格式：
{{"title": "任务标题（动宾短语，15字以内）", "description": "做什么 + 验收要点（1-2句）", "task_type": "development 或 testing 或 bug_fix", "priority": "P0/P1/P2/P3"}}

拆解原则：
1. 输出 3~8 条任务，按模块与依赖顺序排列（先基础后集成）
2. 涉及测试的独立成条（task_type=testing），不要混入开发任务
3. 任务粒度以一名开发 0.5~2 天可完成为准
4. 不要输出与本需求无关的通用任务（如"搭建 CI"）"#,
        title, description, prd
    )
}

/// 从 LLM 输出解析任务数组：兼容裸数组 / `{"tasks": [...]}` 包裹 / markdown 围栏
pub fn parse_breakdown_json(text: &str) -> Vec<BreakdownTask> {
    // 剥掉可能的 ```json ... ``` 围栏
    let stripped = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 候选1：裸数组（截取首个 '[' 到末个 ']'）
    if let (Some(start), Some(end)) = (stripped.find('['), stripped.rfind(']')) {
        if end > start {
            if let Ok(tasks) = serde_json::from_str::<Vec<BreakdownTask>>(&stripped[start..=end]) {
                return tasks;
            }
        }
    }

    // 候选2：对象包裹（复用对象提取，挑含 tasks 数组的）
    for candidate in extract_json_objects(stripped) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
            if let Some(arr) = v.get("tasks").and_then(|t| t.as_array()) {
                if let Ok(tasks) = serde_json::from_value::<Vec<BreakdownTask>>(arr.clone().into()) {
                    return tasks;
                }
            }
        }
    }

    Vec::new()
}

pub fn build_enrich_prompt(title: &str, description: &str) -> String {
    format!(
        r#"你是一个专业的需求分析师。请根据以下需求信息，自动分析并输出结构化的补充信息。

## 需求信息
- 标题: {}
- 描述: {}

## 输出要求
请以JSON格式输出以下字段（只输出JSON，不要其他内容）：
{{
    "module": "所属模块（如：用户管理、订单系统、支付模块等）",
    "tags": "标签（逗号分隔，如：前端,UI,交互）",
    "acceptance_criteria": "验收标准（简要描述）",
    "business_priority": "业务优先级（critical/high/medium/low）",
    "estimated_effort": "预估工作量（人天，数字）",
    "requirement_type": "需求类型（feature/bug/optimization/tech_debt）",
    "source": "需求来源（product/tech/customer/internal）"
}}"#,
        title, description
    )
}

/// 从 LLM 输出中提取全部**平衡且可解析**的 JSON 对象片段。
///
/// 场景（QClaw 环境实测）：stdout 会混入 CLI 自身的告警行
/// `[claude-code:unrecognized_model] {"model":"...",...}`，
/// 旧的"第一个 `{` 到最后一个 `}`"策略会横跨两个对象导致解析失败、回填全空。
/// 平衡扫描逐个产出候选，调用方按业务字段挑选正确的那个。
pub fn extract_json_objects(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut start: Option<usize> = None;
    let mut in_str = false;
    let mut escaped = false;

    for (i, &b) in bytes.iter().enumerate() {
        let c = b as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let candidate = &text[s..=i];
                        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                            out.push(candidate);
                        }
                    }
                    start = None;
                }
                if depth < 0 {
                    depth = 0;
                }
            }
            _ => {}
        }
    }

    out
}

/// 解析"需求智能完善"结果：在全部候选 JSON 中挑选**含业务字段**（module/tags/
/// acceptance_criteria 任一）的对象；都不含时返回空骨架（保持原 fallback 行为）。
///
/// 背景（QClaw 环境实测）：CLI stdout 会先输出告警 JSON
/// `{"model":"mimo-v2.6-flash",...}` 再输出真正的完善结果，
/// 单一"首个对象"策略会错拿告警导致回填全空。
pub fn parse_enriched_json(text: &str) -> serde_json::Value {
    const KEY_FIELDS: [&str; 3] = ["module", "tags", "acceptance_criteria"];

    for candidate in extract_json_objects(text) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
            if KEY_FIELDS.iter().any(|k| v.get(*k).is_some()) {
                return v;
            }
        }
    }

    serde_json::json!({
        "module": "",
        "tags": "",
        "acceptance_criteria": "",
        "business_priority": "medium",
        "estimated_effort": 0,
        "requirement_type": "feature",
        "source": "product"
    })
}

/// Claude API 客户端
pub struct ClaudeClient {
    api_key: String,
    client: Client,
    /// 使用的模型名；可通过 `with_model` 覆盖（对应 `settings.llm_model`）
    model: String,
}

/// 默认模型（未配置 `settings.llm_model` 时使用）
pub const DEFAULT_MODEL: &str = "claude-sonnet-5-20250514";

/// Claude API 响应结构
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    model: String,
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    #[serde(rename = "input_tokens")]
    input_tokens: u32,
    #[serde(rename = "output_tokens")]
    output_tokens: u32,
}

/// Token使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model: String,
}

/// 单条 SSE 行的解析结果
enum SseOutcome {
    /// 增量文本片段
    Delta(String),
    /// 无需处理（心跳、其它事件类型、非 data 行等）
    Ignore,
    /// 流结束标记（`data: [DONE]`）
    Done,
}

/// 解析一条 Anthropic SSE 行，并把 Token / 模型信息累加到入参上。
fn parse_sse_line(
    line: &str,
    model: &mut String,
    input_tokens: &mut u32,
    output_tokens: &mut u32,
) -> SseOutcome {
    let Some(data) = line.strip_prefix("data: ") else {
        return SseOutcome::Ignore;
    };
    if data.trim() == "[DONE]" {
        return SseOutcome::Done;
    }
    let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
        return SseOutcome::Ignore;
    };

    match event.get("type").and_then(|v| v.as_str()) {
        Some("content_block_delta") => event
            .get("delta")
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str())
            .map(|t| SseOutcome::Delta(t.to_string()))
            .unwrap_or(SseOutcome::Ignore),
        Some("message_delta") => {
            if let Some(usage) = event.get("usage") {
                *output_tokens = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
            }
            SseOutcome::Ignore
        }
        Some("message_start") => {
            if let Some(msg) = event.get("message") {
                if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                    *model = m.to_string();
                }
                if let Some(usage) = msg.get("usage") {
                    *input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                }
            }
            SseOutcome::Ignore
        }
        _ => SseOutcome::Ignore,
    }
}

impl ClaudeClient {
    /// 创建新的Claude客户端（带超时设置）
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))  // 2分钟超时
            .connect_timeout(Duration::from_secs(10))  // 10秒连接超时
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            client,
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// 覆盖使用的模型（空字符串会被忽略，保持默认模型）
    pub fn with_model(mut self, model: Option<&str>) -> Self {
        if let Some(m) = model {
            let trimmed = m.trim();
            if !trimmed.is_empty() {
                self.model = trimmed.to_string();
            }
        }
        self
    }

    /// 生成PRD文档
    pub async fn generate_prd(&self, title: &str, description: &str, priority: &str, skill_template: Option<&str>) -> Result<(String, TokenStats), String> {
        debug!("构建PRD生成提示词...");

        let prompt = build_prd_prompt(title, description, priority, skill_template);

        info!("调用Claude API...");
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": 4096,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| {
                error!("API请求失败: {}", e);
                format!("API请求失败: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("API返回错误: status={}, body={}", status, error_text);
            return Err(format!("API返回错误: {}", error_text));
        }

        debug!("解析API响应...");
        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("解析响应失败: {}", e);
                format!("解析响应失败: {}", e)
            })?;

        let text = claude_response.content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let stats = TokenStats {
            input_tokens: claude_response.usage.as_ref().map(|u| u.input_tokens).unwrap_or(0),
            output_tokens: claude_response.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0),
            model: claude_response.model,
        };

        info!("PRD生成完成: model={}, input_tokens={}, output_tokens={}",
            stats.model, stats.input_tokens, stats.output_tokens);

        Ok((text, stats))
    }

    /// 流式生成PRD文档（SSE streaming）
    pub async fn generate_prd_streaming(
        &self,
        title: &str,
        description: &str,
        priority: &str,
        skill_template: Option<&str>,
        event_sender: mpsc::Sender<String>,
    ) -> Result<(String, TokenStats), String> {
        let prompt = if let Some(template) = skill_template {
            let filled_template = template
                .replace("{title}", title)
                .replace("{description}", description)
                .replace("{priority}", priority);

            format!(
                r#"你是一个专业的产品经理。请根据以下需求信息和模板，生成一份结构化的PRD文档。

## 需求信息
- 标题: {}
- 描述: {}
- 优先级: {}

## PRD 模板
请按照以下模板结构生成PRD内容：

{}请直接输出PRD内容，不要包含其他说明。"#,
                title, description, priority, filled_template
            )
        } else {
            format!(
                r#"你是一个专业的产品经理。请根据以下需求描述，生成一份结构化的PRD文档。

## 需求信息
- 标题: {}
- 描述: {}
- 优先级: {}

## 输出要求
请生成包含以下章节的PRD文档（使用Markdown格式）：

### 1. 需求背景与目标
### 2. 用户故事
### 3. 功能描述
### 4. 验收标准
### 5. 技术要点
### 6. 风险评估

请直接输出PRD内容，不要包含其他说明。"#,
                title, description, priority
            )
        };

        info!("调用Claude API（流式）...");
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": 4096,
                "stream": true,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("API请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API返回错误 ({}): {}", status, error_text));
        }

        // 真实流式解析：逐块读取响应体，每解析出一个增量片段立即推送。
        // 与旧实现（先 `response.text()` 取全文再按行解析）不同，这里具备真实的实时性。
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;
        let mut model = String::new();
        let mut finished = false;

        while !finished {
            let chunk = match stream.next().await {
                Some(Ok(chunk)) => chunk,
                Some(Err(e)) => return Err(format!("读取流失败: {}", e)),
                None => break,
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // SSE 以行分隔；只处理已完整到达的行，残片留在 buffer 等待后续块
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end_matches('\r').to_string();
                buffer.drain(..=pos);

                match parse_sse_line(&line, &mut model, &mut input_tokens, &mut output_tokens) {
                    SseOutcome::Delta(text) => {
                        full_text.push_str(&text);
                        // 推送失败说明接收端已关闭，无需继续读取
                        if event_sender.send(text).await.is_err() {
                            finished = true;
                            break;
                        }
                    }
                    SseOutcome::Done => {
                        finished = true;
                        break;
                    }
                    SseOutcome::Ignore => {}
                }
            }
        }

        let stats = TokenStats { input_tokens, output_tokens, model };
        info!("流式PRD生成完成: input_tokens={}, output_tokens={}", input_tokens, output_tokens);

        Ok((full_text, stats))
    }

    /// 通用一次性补全（Skill 自进化：Judge 评分 / 优化器草稿共用）。
    ///
    /// 与 PRD 通道完全独立的 prompt 构造路径：`system` 仅承载角色与输出格式约束，
    /// `user` 携带待评估/待优化内容，降低模板内容注入评分指令的风险。
    pub async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> Result<(String, TokenStats), String> {
        debug!("调用 Claude API complete: max_tokens={}", max_tokens);
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": max_tokens,
                "system": system,
                "messages": [{ "role": "user", "content": user }]
            }))
            .send()
            .await
            .map_err(|e| format!("API请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API返回错误 ({}): {}", status, error_text));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let text = claude_response.content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();
        let stats = TokenStats {
            input_tokens: claude_response.usage.as_ref().map(|u| u.input_tokens).unwrap_or(0),
            output_tokens: claude_response.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0),
            model: claude_response.model,
        };
        Ok((text, stats))
    }

    /// 需求自动完善：根据需求描述自动填充扩展字段。
    ///
    /// 返回解析后的字段建议与本次调用的 Token 统计（用于成本记录）。
    pub async fn enrich_requirement(
        &self,
        title: &str,
        description: &str,
    ) -> Result<(serde_json::Value, TokenStats), String> {
        let prompt = build_enrich_prompt(title, description);

        info!("调用Claude API完善需求...");
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": 1024,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("API请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API返回错误 ({}): {}", status, error_text));
        }

        let claude_response: ClaudeResponse = response.json().await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let text = claude_response.content.first().map(|c| c.text.clone()).unwrap_or_default();

        // 尝试解析JSON（可能被 markdown 包裹或混入告警行，逐候选挑含业务字段的）
        let enriched = parse_enriched_json(&text);

        info!("需求完善完成");

        let usage = claude_response.usage.unwrap_or(ClaudeUsage {
            input_tokens: 0,
            output_tokens: 0,
        });
        let stats = TokenStats {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            model: claude_response.model,
        };

        Ok((enriched, stats))
    }

    /// 智能拆解：需求+PRD → 候选任务清单（API 通道，P1-⑪）
    pub async fn breakdown_tasks(
        &self,
        title: &str,
        description: &str,
        prd: &str,
    ) -> Result<(Vec<BreakdownTask>, TokenStats), String> {
        let prompt = build_breakdown_prompt(title, description, prd);

        info!("调用Claude API智能拆解...");
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": 2048,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("API请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API返回错误 ({}): {}", status, error_text));
        }

        let claude_response: ClaudeResponse = response.json().await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let text = claude_response.content.first().map(|c| c.text.clone()).unwrap_or_default();
        let tasks = sanitize_breakdown_tasks(parse_breakdown_json(&text));
        if tasks.is_empty() {
            return Err("LLM 未返回可解析的任务清单，请重试或补充 PRD 后再拆解".to_string());
        }

        info!("智能拆解完成: {} 条任务", tasks.len());

        let usage = claude_response.usage.unwrap_or(ClaudeUsage {
            input_tokens: 0,
            output_tokens: 0,
        });
        let stats = TokenStats {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            model: claude_response.model,
        };

        Ok((tasks, stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// QClaw 环境实测场景：stdout 先输出告警 JSON 再输出真正的完善结果，
    /// 旧的"首个 `{` 到末个 `}`"策略会横跨两个对象导致解析失败、回填全空。
    #[test]
    fn parse_enriched_skips_warning_json_before_result() {
        let text = concat!(
            "[claude-code:unrecognized_model] {\"model\":\"mimo-v2.6-flash\",\"query_source\":\"sdk\"}\n",
            "{\n",
            "    \"module\": \"用户管理\",\n",
            "    \"tags\": \"前端,后端\",\n",
            "    \"acceptance_criteria\": \"登录成功\",\n",
            "    \"business_priority\": \"critical\",\n",
            "    \"estimated_effort\": 5,\n",
            "    \"requirement_type\": \"feature\",\n",
            "    \"source\": \"product\"\n",
            "}"
        );
        let v = parse_enriched_json(text);
        assert_eq!(v["module"], "用户管理");
        assert_eq!(v["tags"], "前端,后端");
        assert_eq!(v["estimated_effort"], 5);
    }

    #[test]
    fn parse_enriched_handles_markdown_wrapped_json() {
        let text = "以下是分析结果：\n```json\n{\"module\":\"订单系统\",\"tags\":\"后端\"}\n```";
        let v = parse_enriched_json(text);
        assert_eq!(v["module"], "订单系统");
    }

    #[test]
    fn parse_enriched_falls_back_when_no_business_fields() {
        let v = parse_enriched_json("没有任何有效JSON {\"a\":1}");
        assert_eq!(v["module"], "");
        assert_eq!(v["business_priority"], "medium");
    }

    #[test]
    fn extract_json_objects_handles_nested_and_multiple() {
        let text = r#"{"model":"x"} prefix {"a":{"b":1},"module":"m"}"#;
        let all = extract_json_objects(text);
        assert_eq!(all.len(), 2);
        assert!(all[1].contains("\"module\""));
    }
}
