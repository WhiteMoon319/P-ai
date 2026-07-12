export type UpdateRuntimeKind = "installer" | "portable";

export type GithubUpdateInfo = {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseUrl: string;
  updateSource: string;
  accessMode: "direct" | "proxy";
  releaseNotes: string;
  publishedAt?: string;
  runtimeKind: UpdateRuntimeKind;
  canForceUpdate: boolean;
};

export type UpdateProgressPayload = {
  stage: string;
  message: string;
  runtimeKind: UpdateRuntimeKind;
  currentVersion?: string;
  targetVersion?: string;
  downloadedBytes?: number;
  contentLength?: number;
  percent?: number;
  error?: string;
};

export type GithubUpdateState = {
  stage: string;
  currentVersion: string;
  latestVersion: string;
  runtimeKind: UpdateRuntimeKind;
  hasPreparedUpdate: boolean;
  hasVisibleUpdate: boolean;
  releaseNotes: string;
  releaseUrl: string;
  publishedAt?: string;
  preparedAt?: string;
  lastCheckedAt?: string;
  lastError?: string;
  skippedVersion: string;
};
