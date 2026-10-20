import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { I18nProvider, useI18n, LOCALES } from "../i18n";

function Probe() {
  const { locale, setLocale, t } = useI18n();
  return (
    <div>
      <span data-testid="locale">{locale}</span>
      <span data-testid="nav">{t("nav.settings", "设置")}</span>
      <span data-testid="missing">{t("not.translated.key", "中文原文")}</span>
      <button onClick={() => setLocale("en-US")}>to-en</button>
    </div>
  );
}

describe("i18n 渐进式框架", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("默认中文：t 回退中文原文", () => {
    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>,
    );
    expect(screen.getByTestId("locale").textContent).toBe("zh-CN");
    expect(screen.getByTestId("nav").textContent).toBe("设置");
  });

  it("切换英文：有翻译用英文，无翻译回退中文", () => {
    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>,
    );
    act(() => {
      screen.getByText("to-en").click();
    });
    expect(screen.getByTestId("nav").textContent).toBe("Settings");
    expect(screen.getByTestId("missing").textContent).toBe("中文原文");
    expect(localStorage.getItem("ace.locale")).toBe("en-US");
  });

  it("语言选择持久化到 localStorage", () => {
    localStorage.setItem("ace.locale", "en-US");
    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>,
    );
    expect(screen.getByTestId("locale").textContent).toBe("en-US");
  });

  it("提供中英两个语言选项", () => {
    expect(LOCALES.map((l) => l.value)).toEqual(["zh-CN", "en-US"]);
  });
});
