import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MaintenanceToolsPanel } from "@/screens";
import type { AppActions } from "@/lib/actions";
import type { BackendSettings, SettingsResult } from "@/types";

// 注入到 Codex 窗口里的那张控制台页面删掉以后，这几个开关只剩这里能改。
// 这份测试守住「开关还在」和「点了写的是同一份设置」这两件事。
function backendSettings(overrides: Partial<BackendSettings> = {}) {
  return {
    codexAppPath: "",
    codexExtraArgs: [],
    providerSyncEnabled: true,
    providerSyncSavedProviders: [],
    providerSyncManualProviders: [],
    providerSyncLastSelectedProvider: "",
    relayProfilesEnabled: true,
    enhancementsEnabled: true,
    computerUseGuardEnabled: false,
    claudeDesktopComputerUseEnabled: false,
    codexAppPluginEntryUnlock: true,
    codexAppPluginMarketplaceUnlock: false,
    codexAppForcePluginInstall: false,
    codexAppSessionDelete: false,
    codexAppMarkdownExport: false,
    codexAppProjectMove: false,
    codexAppConversationTimeline: true,
    codexAppConversationView: false,
    codexAppThreadScrollRestore: true,
    codexAppZedRemoteOpen: false,
    codexAppUpstreamWorktreeCreate: false,
    codexAppNativeMenuPlacement: false,
    claudeAppChineseOverlayEnabled: false,
    codexAppServiceTierControls: true,
    codexAppImageOverlayEnabled: false,
    codexAppImageOverlayPath: "",
    launchMode: "patch",
    ...overrides,
  } as unknown as BackendSettings;
}

function settingsResult(settings: BackendSettings): SettingsResult {
  return {
    status: "ok",
    message: "",
    settings,
    settings_path: "",
    user_scripts: {} as SettingsResult["user_scripts"],
  } as unknown as SettingsResult;
}

function actionsWith(saveSettings: AppActions["saveSettings"]) {
  return { saveSettings } as unknown as AppActions;
}

describe("Codex injection switches", () => {
  it("keeps the five switches and the service mode in the settings page", () => {
    render(<MaintenanceToolsPanel actions={actionsWith(vi.fn())} settings={settingsResult(backendSettings())} />);

    for (const title of ["页面功能增强", "插件市场解锁", "强制解锁入口", "特殊插件强制安装", "Fast 按钮"]) {
      expect(screen.getByRole("switch", { name: title })).toBeTruthy();
    }
    expect(screen.getByRole("radiogroup", { name: "服务模式" })).toBeTruthy();
    expect(screen.getByRole("switch", { name: "插件市场解锁" }).getAttribute("aria-checked")).toBe("false");
    expect(screen.getByRole("switch", { name: "页面功能增强" }).getAttribute("aria-checked")).toBe("true");
  });

  it("saves the whole settings object with the flipped key", async () => {
    const saveSettings = vi.fn(async () => null);
    render(<MaintenanceToolsPanel actions={actionsWith(saveSettings)} settings={settingsResult(backendSettings())} />);

    fireEvent.click(screen.getByRole("switch", { name: "插件市场解锁" }));

    await waitFor(() => expect(saveSettings).toHaveBeenCalledTimes(1));
    const saved = (saveSettings.mock.calls[0] as unknown as [BackendSettings])[0];
    expect(saved.codexAppPluginMarketplaceUnlock).toBe(true);
    // 其余键必须原样带回，否则一次点开关会把别的设置清空。
    expect(saved.enhancementsEnabled).toBe(true);
    expect(saved.codexAppServiceTierControls).toBe(true);
    expect(saved.launchMode).toBe("patch");
  });

  it("switches the service mode between patch and relay", async () => {
    const saveSettings = vi.fn(async () => null);
    render(<MaintenanceToolsPanel actions={actionsWith(saveSettings)} settings={settingsResult(backendSettings())} />);

    fireEvent.click(screen.getByRole("radio", { name: "兼容增强" }));

    await waitFor(() => expect(saveSettings).toHaveBeenCalledTimes(1));
    const saved = (saveSettings.mock.calls[0] as unknown as [BackendSettings])[0];
    expect(saved.launchMode).toBe("relay");
    expect(saved.codexAppPluginMarketplaceUnlock).toBe(false);
  });

  it("says so when the settings have not loaded yet", () => {
    render(<MaintenanceToolsPanel actions={actionsWith(vi.fn())} settings={null} />);
    expect(screen.getByText("正在读取设置，稍等一下再改开关。")).toBeTruthy();
  });
});