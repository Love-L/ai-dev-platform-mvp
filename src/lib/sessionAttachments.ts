/**
 * 会话附件 marker 解析（P1-10）。
 *
 * SessionInput 发送时把附件路径注入消息文本的固定 marker 区块；
 * 气泡渲染侧（MessageBubble）解析该区块为"正文 + 附件列表"。
 * 独立成 lib 以保持组件文件仅导出组件（fast-refresh 约束）。
 */

/** 附件 marker 前缀（与 SessionInput 组装格式保持一致） */
export const ATTACHMENT_MARKER = "📎 附件（请读取并识别以下文件）：";
/** 旧版 marker（历史消息兼容） */
export const ATTACHMENT_MARKER_LEGACY = "📎 附件已保存到本地，请读取并识别以下文件：";

/** 图片扩展名（气泡内嵌缩略图判定） */
export const IMAGE_EXT = /\.(png|jpe?g|gif|webp|bmp)$/i;

export interface ParsedAttachments {
  /** 去掉附件区块后的正文 */
  body: string;
  paths: string[];
}

/** 解析 user 消息中的附件 marker 区块 */
export function parseAttachmentBlock(text: string): ParsedAttachments {
  const idx = text.indexOf(ATTACHMENT_MARKER);
  const legacyIdx = text.indexOf(ATTACHMENT_MARKER_LEGACY);
  const markerIdx = idx >= 0 ? idx : legacyIdx;
  if (markerIdx < 0) return { body: text, paths: [] };

  const body = text.slice(0, markerIdx).trimEnd();
  const block = text.slice(markerIdx);
  const paths = block
    .split("\n")
    .slice(1)
    .map((line) => line.trim().replace(/^[-•]\s*/, ""))
    .filter((line) => line.length > 0 && !line.startsWith("📎"));
  return { body, paths };
}
