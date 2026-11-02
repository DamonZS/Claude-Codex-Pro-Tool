import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { invokeCommand } from "@/tauriBridge";

import { AgentProviderPanel } from "./AgentProviderPanel";

vi.mock("@/tauriBridge", () => ({ invokeCommand: vi.fn() }));

const list = vi.mocked(invokeCommand);

beforeEach(() => {
  list.mockReset();
  list.mockResolvedValue({ status: "ok", message: "", providers: [], states: {} });
});

/** 打开编辑器；返回「接口格式」那个下拉框。 */
async function openFormatPicker() {
  render(<AgentProviderPanel appId="workbuddy" />);
  const add = await screen.findByRole("button", { name: /添加供应商/ });
  fireEvent.click(add);
  const select = await screen.findByLabelText("接口格式");
  return select as HTMLSelectElement;
}

describe("只认 chat 的 Agent 的接口格式", () => {
  it("offers the formats the backend accepts, not just chat", async () => {
    const select = await openFormatPicker();

    // Rust 那边 workbuddy 现在认三种格式（选了非 chat 的由本机代理替它转）；
    // 前端这张表要是落后一步，用户就根本选不到 —— 这条就是盯着它别落后。
    const values = Array.from(select.options).map((option) => option.value);
    expect(values).toEqual(["openai-chat", "anthropic", "openai-responses"]);
  });

  it("says the address becomes CCP's own proxy when the format is not chat", async () => {
    const select = await openFormatPicker();

    expect(screen.queryByText(/由代理替它转成上游要的那套协议/)).toBeNull();

    fireEvent.change(select, { target: { value: "anthropic" } });
    await waitFor(() =>
      expect(screen.getByText(/由代理替它转成上游要的那套协议/)).toBeTruthy(),
    );

    fireEvent.change(select, { target: { value: "openai-chat" } });
    await waitFor(() => expect(screen.queryByText(/由代理替它转成上游要的那套协议/)).toBeNull());
  });
});