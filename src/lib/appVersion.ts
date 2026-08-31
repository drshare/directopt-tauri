import pkg from "../../package.json";

/**
 * 应用发布版本号（单一来源：package.json `version`）。
 *
 * 与 tauri.conf.json / Cargo.toml / git tag 保持一致，
 * 用于顶栏版本徽标、原生窗口标题与文档标题，避免写死导致漂移。
 */
export const APP_VERSION = (pkg as { version: string }).version;

/** 完整窗口/页面标题，例如「绿电直连新能源优化配置软件 V0.0.2」 */
export const APP_TITLE = `绿电直连新能源优化配置软件 V${APP_VERSION}`;
