import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  installCcpBotLatest,
  loadCcpBot,
  prepareCcpBot,
  probeCcpBot,
  readCcpBotLog,
  saveCcpBotConfig,
  startCcpBot,
  stopCcpBot,
} from "@/lib/ccpbot";
import type { CcpBotConfig } from "@/types";

const { invokeCommand } = vi.hoisted(() => ({ invokeCommand: vi.fn() }));

vi.mock("@/tauriBridge", () => ({ invokeCommand }));

const CONFIG: CcpBotConfig = { address: "http://127.0.0.1:3010", mode: "source", projectDir: "H:\\CCP-Bot" };

// The command names and the payload shape are the contract with the Rust side:
// `generate_handler!` in src-tauri/src/lib.rs and these wrappers have to agree, and
// a rename on one side is a silent break that only shows up when a person clicks.
beforeEach(() => {
  invokeCommand.mockReset();
  invokeCommand.mockImplementation(async () => ({ message: "ok", status: "ok" }));
});

describe("ccpbot 命令封装", () => {
  it("读取与探测不带参数", async () => {
    await loadCcpBot();
    expect(invokeCommand).toHaveBeenNthCalledWith(1, "ccpbot_load");

    await probeCcpBot();
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "ccpbot_probe");
    expect(invokeCommand).toHaveBeenCalledTimes(2);
  });

  it("保存、准备、启动、停止都按参数名传整份配置或空", async () => {
    await saveCcpBotConfig(CONFIG);
    expect(invokeCommand).toHaveBeenNthCalledWith(1, "ccpbot_save_config", { config: CONFIG });

    await prepareCcpBot(CONFIG);
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "ccpbot_prepare", { config: CONFIG });

    await startCcpBot();
    expect(invokeCommand).toHaveBeenNthCalledWith(3, "ccpbot_start");

    await stopCcpBot();
    expect(invokeCommand).toHaveBeenNthCalledWith(4, "ccpbot_stop");
  });

  it("一键安装不带地址也不带目录——那台机器本来就没有", async () => {
    await installCcpBotLatest();
    expect(invokeCommand).toHaveBeenCalledWith("ccpbot_install_latest");
    expect(invokeCommand).toHaveBeenCalledTimes(1);
  });

  it("读日志的条数包在 request 里，默认 60、可覆盖", async () => {
    await readCcpBotLog();
    expect(invokeCommand).toHaveBeenNthCalledWith(1, "ccpbot_log_tail", { request: { lines: 60 } });

    await readCcpBotLog(240);
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "ccpbot_log_tail", { request: { lines: 240 } });
  });
});