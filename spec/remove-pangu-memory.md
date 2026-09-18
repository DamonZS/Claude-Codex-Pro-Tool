# Remove Pangu Memory

## Background

The CCP product no longer needs the Pangu Memory feature. The feature currently spans core storage and capture logic, Manager UI and Tauri commands, Codex renderer injection, launcher integration, and the standalone MCP application.

## Goal

Remove the Pangu Memory product capability from CCP, including its public UI, settings, commands, injection behavior, launcher bridge, core module, tests, and standalone MCP crate. Existing user data files such as `memory_assist.sqlite` must not be deleted or modified by this change.

## Requirements

- Manager must contain no Pangu Memory route, screen, settings, actions, types, or Tauri commands.
- Core and launcher must contain no Pangu Memory module integration.
- Codex renderer injection must not create memory badges, panels, capture requests, or memory settings.
- The standalone Pangu Memory MCP application and workspace membership must be removed.
- Windows upgrade and uninstall remove only the retired MCP executable from the installation directory, scheduling deletion on reboot if it is in use. This compatibility cleanup must not touch user databases or configuration.
- Unrelated Codex, Claude, supplier, plugin, session, and launcher behavior remains unchanged.
- Existing local memory database files are preserved; no migration or deletion command is added.

## Delivery

Update source, tests, release checks, and documentation references required to keep the workspace buildable. Add regression assertions proving the removed public surfaces are absent.
