import { invokeCommand } from "@/tauriBridge";
import type {
  CcpBotAction,
  CcpBotConfig,
  CcpBotConfigResult,
  CcpBotLogResult,
  CcpBotSnapshot,
  CcpBotStatus,
  CommandResult,
} from "@/types";

// Thin typed wrappers over the seven `ccpbot_*` Tauri commands. The screen owns its
// own loading instead of going through `AppActions`: CCP-Bot is an embedded,
// self-refreshing surface, and threading its state through the app shell would
// re-render every screen in the workspace on each probe.

export function loadCcpBot() {
  return invokeCommand<CommandResult<CcpBotSnapshot>>("ccpbot_load");
}

export function probeCcpBot() {
  return invokeCommand<CommandResult<CcpBotStatus>>("ccpbot_probe");
}

export function saveCcpBotConfig(config: CcpBotConfig) {
  return invokeCommand<CommandResult<CcpBotConfigResult>>("ccpbot_save_config", { config });
}

// Stage 2: clone the published CCP-Bot repository, install dependencies and seed
// .env. Slow on purpose (minutes on a cold machine) and reported step by step in
// the returned message, because a progress-less spinner reads as a hang.
export function prepareCcpBot(config: CcpBotConfig) {
  return invokeCommand<CommandResult<CcpBotAction>>("ccpbot_prepare", { config });
}

export function startCcpBot() {
  return invokeCommand<CommandResult<CcpBotAction>>("ccpbot_start");
}

export function stopCcpBot() {
  return invokeCommand<CommandResult<CcpBotAction>>("ccpbot_stop");
}

export function readCcpBotLog(lines = 60) {
  return invokeCommand<CommandResult<CcpBotLogResult>>("ccpbot_log_tail", { request: { lines } });
}