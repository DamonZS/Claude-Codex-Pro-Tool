import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CcpBotScreen } from "@/components/CcpBotScreen";
import type {
  CcpBotConfig,
  CcpBotInstallation,
  CcpBotProbe,
  CcpBotRuntime,
  CcpBotSnapshot,
} from "@/types";

const { invokeCommand } = vi.hoisted(() => ({ invokeCommand: vi.fn() }));
const { open } = vi.hoisted(() => ({ open: vi.fn() }));

vi.mock("@/tauriBridge", () => ({ invokeCommand }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));

const ADDRESS = "http://127.0.0.1:3010";
const REFUSED = "连接被拒：该地址没有服务在监听，CCP-Bot 估计还没启动。";
const DEFAULT_INSTALL_DIR = "C:\\Users\\you\\AppData\\Local\\CCP-Bot";

const NOTHING_INSTALLED: CcpBotInstallation = {
  desktopPath: null,
  desktopVersion: null,
  image: null,
  sourceVersion: null,
  uninstallPath: null,
};

type SnapshotOptions = {
  address?: string;
  mode?: CcpBotConfig["mode"];
  projectDir?: string;
  reachable?: boolean;
  status?: number | null;
  detail?: string;
  bunVersion?: string | null;
  dockerDaemon?: boolean;
  envReady?: boolean;
  installed?: Partial<CcpBotInstallation>;
  defaultInstallDir?: string;
  pid?: number | null;
  logPath?: string;
  logText?: string;
};

function snapshot(options: SnapshotOptions = {}): CcpBotSnapshot {
  const {
    address = ADDRESS,
    mode = "source",
    projectDir = "",
    reachable = false,
    status = null,
    detail = REFUSED,
    bunVersion = "1.2.21",
    dockerDaemon = false,
    envReady = false,
    installed = {},
    defaultInstallDir = DEFAULT_INSTALL_DIR,
    pid = null,
    logPath = "",
    logText = "",
  } = options;
  const runtime: CcpBotRuntime = {
    appPortOpen: false,
    bunVersion,
    dockerCli: mode === "docker" ? "29.8.2" : null,
    dockerDaemon,
    envReady,
    projectDirPresent: projectDir !== "",
    serverPortOpen: false,
  };
  const probe: CcpBotProbe = { address, detail, elapsedMs: 21, reachable, status };
  return {
    config: { address, mode, projectDir },
    defaultInstallDir,
    installed: { ...NOTHING_INSTALLED, ...installed },
    logPath,
    logText,
    pid,
    probe,
    runtime,
  };
}

function reply(payload: Record<string, unknown>, message = "已就绪。"): Record<string, unknown> {
  return { message, status: "ok", ...payload };
}

beforeEach(() => {
  invokeCommand.mockReset();
  open.mockReset();
});

describe("CCP-Bot 界面", () => {
  it("空状态点名缺的那几步，并给出对应的一键操作", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") return reply({ ...snapshot() });
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);

    expect(await screen.findByText("CCP-Bot 还没跑起来")).toBeTruthy();
    // The probe's own sentence is shown verbatim: it is already the classified,
    // actionable explanation, and a paraphrase would be a downgrade.
    expect(screen.getByText(REFUSED)).toBeTruthy();
    expect(screen.getByText("取得 CCP-Bot")).toBeTruthy();
    expect(screen.getByText("本机 bun")).toBeTruthy();
    expect(screen.getByText("bun 1.2.21")).toBeTruthy();
    expect(screen.getByText("配置")).toBeTruthy();
    expect(screen.getByText("缺 .env：把 .env.example 复制成 .env")).toBeTruthy();
    expect(screen.getByText("还差 2 步：")).toBeTruthy();
    expect(screen.getByRole("button", { name: /启动 CCP-Bot/ })).toBeTruthy();
    // Two ways to re-ask: the rail's icon and the empty state's own action.
    // Two ways to re-ask: the rail's icon and the empty state's own action.
    expect(screen.getAllByRole("button", { name: /重新检测/ })).toHaveLength(2);
    expect(screen.getByRole("button", { name: /一键安装/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /下载并准备/ })).toBeTruthy();
    expect(document.querySelector("iframe")).toBeNull();
  });

  it("可达时把舞台交给内嵌界面，并且不再显示清单", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({
          ...snapshot({
            detail: "CCP-Bot 已响应。",
            envReady: true,
            pid: 4172,
            projectDir: "H:\\CCP-Bot",
            reachable: true,
            status: 200,
          }),
        });
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);

    const frame = await waitFor(() => {
      const found = document.querySelector("iframe");
      if (!found) throw new Error("frame not mounted yet");
      return found as HTMLIFrameElement;
    });
    expect(frame.getAttribute("src")).toBe(ADDRESS);
    expect(frame.getAttribute("title")).toBe("CCP-Bot");
    expect(frame.getAttribute("referrerpolicy")).toBe("no-referrer");
    expect(screen.getByText("在线")).toBeTruthy();
    expect(screen.getByText("源码")).toBeTruthy();
    expect(screen.getByText("4172")).toBeTruthy();
    expect(screen.getByText("内嵌界面")).toBeTruthy();
    expect(document.querySelector(".ccpbot-steps")).toBeNull();
  });

  it("Docker 模式读的是 daemon，不是 bun", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({
          ...snapshot({ bunVersion: null, detail: "连接超时。", dockerDaemon: false, mode: "docker", projectDir: "H:\\CCP-Bot" }),
        });
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);

    expect(await screen.findByText("Docker 引擎")).toBeTruthy();
    expect(screen.getByText("daemon 未启动，先打开 Docker Desktop")).toBeTruthy();
    expect(screen.getByText("未启动")).toBeTruthy();
    // Compose brings its own environment: in that mode .env is not a step.
    expect(screen.getByText("compose 自带环境变量")).toBeTruthy();
    expect(screen.getByText("还差 1 步：")).toBeTruthy();
  });

  it("缺引擎但有目录时，启动按钮被挡住并说明原因", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({ ...snapshot({ bunVersion: null, projectDir: "H:\\CCP-Bot" }) });
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);

    const start = (await screen.findByRole("button", { name: /启动 CCP-Bot/ })) as HTMLButtonElement;
    expect(start.disabled).toBe(true);
    expect(start.getAttribute("title")).toBe("先补上缺的那一步");
  });

  it("按下启动会真的启动、等一会儿、再探测成内嵌界面", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({ ...snapshot({ bunVersion: "1.2.21", envReady: true, projectDir: "H:\\CCP-Bot" }) });
      }
      if (command === "ccpbot_start") {
        return reply({ changed: true, logPath: "…/ccpbot-dev.log", message: "已在 3010 上启动。", pid: 4172 });
      }
      if (command === "ccpbot_probe") {
        return reply({
          pid: 4172,
          probe: { address: ADDRESS, detail: "CCP-Bot 已响应。", elapsedMs: 18, reachable: true, status: 200 },
          runtime: snapshot({ reachable: true, status: 200 }).runtime,
        });
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);

    fireEvent.click(await screen.findByRole("button", { name: /启动 CCP-Bot/ }));

    await waitFor(() => expect(document.querySelector("iframe")).toBeTruthy(), { timeout: 8000 });
    expect(invokeCommand).toHaveBeenCalledWith("ccpbot_start");
    expect(invokeCommand).toHaveBeenCalledWith("ccpbot_probe");
    expect(screen.getByText("4172")).toBeTruthy();
  }, 15000);

  it("后端报错不静默：原文进提示，且可以关掉", async () => {
    invokeCommand.mockImplementation(async () => {
      throw new Error("backend offline");
    });

    render(<CcpBotScreen />);

    expect(await screen.findByText(/读取 CCP-Bot 状态失败：Error: backend offline/)).toBeTruthy();
    const toast = screen.getByRole("status");
    expect(toast.className).toContain("warn");

    fireEvent.click(screen.getByRole("button", { name: "关闭提示" }));
    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
  });

  it("连接设置收在面板里，保存时把整份配置交给后端", async () => {
    const saved: unknown[] = [];
    invokeCommand.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "ccpbot_load") return reply({ ...snapshot() });
      if (command === "ccpbot_save_config") {
        saved.push(args);
        return reply({ config: args?.config }, "配置已保存。");
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");

    fireEvent.click(screen.getByRole("button", { name: "连接设置" }));
    const address = screen.getByPlaceholderText(ADDRESS) as HTMLInputElement;
    fireEvent.change(address, { target: { value: "http://127.0.0.1:3020" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(saved).toEqual([
        { config: { address: "http://127.0.0.1:3020", mode: "source", projectDir: "" } },
      ]),
    );
  });

  it("日志抽屉只在被打开时读尾部", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({ ...snapshot({ logPath: "…/ccpbot-dev.log" }) });
      }
      if (command === "ccpbot_probe") {
        return reply({
          pid: null,
          probe: snapshot().probe,
          runtime: snapshot().runtime,
        });
      }
      if (command === "ccpbot_log_tail") {
        return reply({ path: "…/ccpbot-dev.log", text: "$ bun run dev\nready in 3s" }, "日志已读取。");
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");

    fireEvent.click(screen.getByRole("button", { name: "启动日志" }));

    await waitFor(() => expect(invokeCommand).toHaveBeenCalledWith("ccpbot_log_tail", { request: { lines: 120 } }));
    expect(await screen.findByText(/\$ bun run dev/)).toBeTruthy();
  });

  it("打开浏览器走的是既有外链命令，地址取当前配置", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") return reply({ ...snapshot() });
      if (command === "open_external_url") return reply({}, "已交给系统浏览器。");
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");

    fireEvent.click(screen.getByRole("button", { name: "在浏览器打开" }));

    await waitFor(() => expect(invokeCommand).toHaveBeenCalledWith("open_external_url", { url: ADDRESS }));
  });

  it("没装过的机器由一键安装走完全程：先问装到哪，再把读回来的版本显示出来", async () => {
    let reported: Partial<CcpBotInstallation> = {};
    open.mockResolvedValue("D:\\CCP-Bot");
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") return reply({ ...snapshot({ installed: reported }) });
      if (command === "ccpbot_install_latest") {
        reported = {
          desktopPath: "D:\\CCP-Bot\\ccpbot-desktop.exe",
          desktopVersion: "0.2.4",
          uninstallPath: "D:\\CCP-Bot\\uninstall.exe",
        };
        return reply(
          { changed: true, logPath: "…/ccpbot-install.log", message: "已下载并安装 CCP-Bot 0.2.4", pid: null },
          "已下载并安装 CCP-Bot 0.2.4，装到 D:\\CCP-Bot。",
        );
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");
    expect(screen.getByText("本机还没有 CCP-Bot：")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /一键安装/ }));

    // The destination is asked for first, with our own default sitting in the dialog, so the
    // installer never has to guess where a standard account may write.
    await waitFor(() =>
      expect(open).toHaveBeenCalledWith(
        expect.objectContaining({ defaultPath: DEFAULT_INSTALL_DIR, directory: true, multiple: false }),
      ),
    );
    await waitFor(() =>
      expect(invokeCommand).toHaveBeenCalledWith("ccpbot_install_latest", { request: { dir: "D:\\CCP-Bot" } }),
    );
    expect(await screen.findByText("本机已装 CCP-Bot 0.2.4：")).toBeTruthy();
    expect(screen.getByText("桌面版已装 0.2.4")).toBeTruthy();
    // Re-reading the state is what tells the page the machine has changed; the install itself
    // reports a message, not a snapshot.
    expect(invokeCommand.mock.calls.filter(([command]) => command === "ccpbot_load")).toHaveLength(2);
  });

  it("取消目录选择就什么都不装", async () => {
    open.mockResolvedValue(null);
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") return reply({ ...snapshot() });
      if (command === "ccpbot_install_latest") return reply({}, "不该走到这里。");
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");

    fireEvent.click(screen.getByRole("button", { name: /一键安装/ }));

    // A cancelled dialog still goes to the backend as an explicit "no directory": the same call
    // shape as the button, with the installer's own default left to decide.
    await waitFor(() =>
      expect(invokeCommand).toHaveBeenCalledWith("ccpbot_install_latest", { request: null }),
    );
  });

  it("装过的人看到版本，拿到的是重新安装而不是从头下载", async () => {
    invokeCommand.mockImplementation(async (command: string) => {
      if (command === "ccpbot_load") {
        return reply({
          ...snapshot({
            installed: {
              desktopPath: "H:\\CCP-Bot\\ccpbot-desktop.exe",
              desktopVersion: "0.2.4",
              uninstallPath: "H:\\CCP-Bot\\uninstall.exe",
            },
          }),
        });
      }
      throw new Error(`unexpected ${command}`);
    });

    render(<CcpBotScreen />);
    await screen.findByText("CCP-Bot 还没跑起来");

    expect(screen.getByText("本机已装 CCP-Bot 0.2.4：")).toBeTruthy();
    expect(screen.getByText("桌面版已装 0.2.4")).toBeTruthy();
    expect(screen.getByText("已装")).toBeTruthy();
    expect(screen.getByRole("button", { name: /重新安装最新版/ })).toBeTruthy();
    // Nobody who already has it is pushed to download it again.
    expect(screen.queryByRole("button", { name: /一键安装/ })).toBeNull();
    // The one thing still missing on this machine is the configuration, so only that step is left.
    expect(screen.getByText("还差 1 步：")).toBeTruthy();
  });
});