export type CacheCleanupOption = {
  id: string;
  label: string;
  description: string;
  defaultChecked: boolean;
};

export const CACHE_CLEANUP_OPTIONS: CacheCleanupOption[] = [
  {
    id: "user_temp",
    label: "User Temp",
    description: "Clear %LOCALAPPDATA%\\Temp",
    defaultChecked: true,
  },
  {
    id: "windows_temp",
    label: "Windows Temp",
    description: "Clear Windows temporary files",
    defaultChecked: true,
  },
  {
    id: "windows_update_cache",
    label: "Windows Update Cache",
    description: "Clear SoftwareDistribution download cache",
    defaultChecked: true,
  },
  {
    id: "prefetch",
    label: "Prefetch",
    description: "Clear prefetch cache files",
    defaultChecked: false,
  },
  {
    id: "explorer_cache",
    label: "Explorer Cache",
    description: "Clear icon and thumbnail cache",
    defaultChecked: true,
  },
  {
    id: "edge_cache",
    label: "Microsoft Edge Cache",
    description: "Clear Edge browser cache",
    defaultChecked: false,
  },
  {
    id: "chrome_cache",
    label: "Google Chrome Cache",
    description: "Clear Chrome browser cache",
    defaultChecked: false,
  },
  {
    id: "firefox_cache",
    label: "Mozilla Firefox Cache",
    description: "Clear Firefox browser cache",
    defaultChecked: false,
  },
  {
    id: "inet_cache",
    label: "INetCache",
    description: "Clear legacy internet cache",
    defaultChecked: true,
  },
  {
    id: "web_cache",
    label: "WebCache",
    description: "Clear Windows WebCache store",
    defaultChecked: false,
  },
  {
    id: "crash_dumps",
    label: "Crash Dumps",
    description: "Clear local crash dump files",
    defaultChecked: true,
  },
  {
    id: "wer_reports",
    label: "Windows Error Reporting (WER)",
    description: "Clear WER reports and queue",
    defaultChecked: true,
  },
  {
    id: "d3d_shader_cache",
    label: "DirectX Shader Cache",
    description: "Clear D3DSCache",
    defaultChecked: true,
  },
];

export const DEFAULT_CACHE_SELECTION = CACHE_CLEANUP_OPTIONS
  .filter((option) => option.defaultChecked)
  .map((option) => option.id);
