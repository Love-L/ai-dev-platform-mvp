import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { getSetting, setSetting, encryptApiKey, decryptApiKey } from "../../src/lib/tauri";

const mockInvoke = vi.mocked(invoke);

describe("L3 Full-stack: 设置与加密全链路", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-020: 获取设置
  it("L3-020: getSetting 返回值", async () => {
    mockInvoke.mockResolvedValue("https://api.openai.com/v1");

    const result = await getSetting("api_base_url");

    expect(mockInvoke).toHaveBeenCalledWith("get_setting", { key: "api_base_url" });
    expect(result).toBe("https://api.openai.com/v1");
  });

  // L3-021: 保存设置
  it("L3-021: setSetting 调用正确", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await setSetting("api_base_url", "https://api.openai.com/v1");

    expect(mockInvoke).toHaveBeenCalledWith("set_setting", {
      key: "api_base_url",
      value: "https://api.openai.com/v1",
    });
  });

  // L3-022: AES-256-GCM 加密 API Key
  it("L3-022: encryptApiKey 返回加密字符串", async () => {
    const encrypted = "vault:v1:encrypted_base64_data==";
    mockInvoke.mockResolvedValue(encrypted);

    const result = await encryptApiKey("sk-test-key-12345", "my-password");

    expect(mockInvoke).toHaveBeenCalledWith("encrypt_api_key", {
      apiKey: "sk-test-key-12345",
      password: "my-password",
    });
    expect(result).toContain("vault:v1:");
  });

  // L3-023: AES-256-GCM 解密 API Key
  it("L3-023: decryptApiKey 返回原始密钥", async () => {
    mockInvoke.mockResolvedValue("sk-test-key-12345");

    const result = await decryptApiKey("vault:v1:encrypted_base64_data==", "my-password");

    expect(mockInvoke).toHaveBeenCalledWith("decrypt_api_key", {
      encrypted: "vault:v1:encrypted_base64_data==",
      password: "my-password",
    });
    expect(result).toBe("sk-test-key-12345");
  });

  // L3-024: 加密解密往返测试
  it("L3-024: 加密后解密还原原始值", async () => {
    const originalKey = "sk-ant-api03-abcdef1234567890";
    const password = "secure-password-123";
    const encrypted = "vault:v1:base64encoded==";

    // 第一次调用: 加密
    mockInvoke.mockResolvedValueOnce(encrypted);
    const encResult = await encryptApiKey(originalKey, password);
    expect(encResult).toBe(encrypted);

    // 第二次调用: 解密
    mockInvoke.mockResolvedValueOnce(originalKey);
    const decResult = await decryptApiKey(encResult, password);
    expect(decResult).toBe(originalKey);

    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });
});
