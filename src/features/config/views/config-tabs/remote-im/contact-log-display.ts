import type { ChannelLogEntry } from "./types";

type TranslateFn = (key: string, params?: Record<string, unknown>) => string;

export type ContactLogDisplayItem = {
  timestamp: string;
  level: string;
  kind: string;
  title: string;
  summary: string;
  detail?: string;
};

function logField(message: string, key: string): string {
  const match = message.match(new RegExp(`${key}=([\\s\\S]*?)(?=, [a-z_]+=|$)`));
  return String(match?.[1] || "").trim();
}

function currentStateLabel(value: string): string {
  const parts = String(value || "").trim().split("->").map((item) => item.trim()).filter(Boolean);
  return parts[parts.length - 1] || "";
}

function humanName(value: string): string {
  return String(value || "").trim().replace(/\(\d+\)\s*$/, "").trim();
}

function humanId(value: string): string {
  const match = String(value || "").trim().match(/\((\d+)\)\s*$/);
  return String(match?.[1] || "").trim();
}

function boolLabel(value: string, t: TranslateFn): string {
  return value === "是" || value.toLowerCase() === "true" ? t("common.yes") : t("common.no");
}

function decisionLabel(value: string, t: TranslateFn): string {
  if (value === "reply" || value === "reply_async") return t("config.remoteIm.logDecisionReply");
  if (value === "send") return t("config.remoteIm.logDecisionSend");
  if (value === "send_files") return t("config.remoteIm.logDecisionSendFiles");
  if (value === "no_reply") return t("config.remoteIm.logDecisionNoReply");
  if (value === "send_async") return t("config.remoteIm.logDecisionSendAsync");
  return value || t("common.done");
}

function stateSummary(message: string, t: TranslateFn): string {
  const presence = currentStateLabel(logField(message, "presence"));
  const work = currentStateLabel(logField(message, "work"));
  const activate = logField(message, "activate");
  return [
    presence,
    work,
    activate ? (boolLabel(activate, t) === t("common.yes") ? t("config.remoteIm.logActivate") : t("config.remoteIm.logInactive")) : "",
  ].filter(Boolean).join("；");
}

export function buildContactLogDisplayItem(log: ChannelLogEntry, t: TranslateFn): ContactLogDisplayItem | null {
  const message = String(log.message || "").trim();
  if (message.startsWith("[联系人消息] 收到:")) {
    const senderRaw = logField(message, "sender");
    const senderName = humanName(senderRaw) || t("config.remoteIm.otherParty");
    const senderId = humanId(senderRaw);
    const preview = logField(message, "preview") || t("config.remoteIm.receivedMessage");
    const imageCount = Number(logField(message, "image_count") || 0);
    const audioCount = Number(logField(message, "audio_count") || 0);
    const attachmentCount = Number(logField(message, "attachment_count") || 0);
    const extras = [
      imageCount > 0 ? t("config.remoteIm.imageCount", { count: imageCount }) : "",
      audioCount > 0 ? t("config.remoteIm.audioCount", { count: audioCount }) : "",
      attachmentCount > 0 ? t("config.remoteIm.attachmentCount", { count: attachmentCount }) : "",
    ].filter(Boolean);
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: t("config.remoteIm.logKindMessage"),
      title: "",
      summary: `${senderId ? `[${senderName}/${senderId}]` : `[${senderName}]`}${preview}`,
      detail: extras.length > 0 ? extras.join("，") : undefined,
    };
  }
  if (message.startsWith("[联系人消息] 去重跳过:")) {
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindDedup"), title: t("config.remoteIm.logDedupTitle"), summary: logField(message, "preview") || t("config.remoteIm.logDedupSummary") };
  }
  if (message.startsWith("[联系人消息] 过滤跳过:")) {
    const prefix = logField(message, "prefix");
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: t("config.remoteIm.logKindFilter"),
      title: t("config.remoteIm.logFilterTitle"),
      summary: prefix
        ? t("config.remoteIm.logFilterPrefix", { prefix })
        : t("config.remoteIm.logFilterSummary"),
    };
  }
  if (message.startsWith("[联系人消息] 入队:")) {
    return log.level === "warn" || log.level === "error"
      ? { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSystem"), title: t("config.remoteIm.logEnqueueFailed"), summary: logField(message, "reason") || t("config.remoteIm.logEnqueueFailedSummary") }
      : null;
  }
  if (message.startsWith("[联系人状态] 入站判定:")) {
    const reason = logField(message, "reason");
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindStatus"), title: stateSummary(message, t), summary: reason ? t("config.remoteIm.logReason", { reason }) : "" };
  }
  if (message.startsWith("[联系人状态] 历史落地:")) {
    return log.level === "warn" || log.level === "error"
      ? { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSystem"), title: t("config.remoteIm.logHistoryWriteFailed"), summary: logField(message, "reason") || t("config.remoteIm.logHistoryWriteFailedSummary") }
      : null;
  }
  if (message.startsWith("[联系人消息] 发出失败:")) {
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSend"), title: t("config.remoteIm.logSendFailed"), summary: logField(message, "preview") || t("config.remoteIm.logSendContentOmitted"), detail: logField(message, "error") || undefined };
  }
  if (message.startsWith("[联系人消息] 发出跳过:")) {
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSend"), title: t("config.remoteIm.logSendSkipped"), summary: logField(message, "reason") || t("config.remoteIm.logSendSkippedSummary") };
  }
  if (message.startsWith("[联系人消息] 发出:")) {
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSend"), title: "", summary: t("config.remoteIm.logSentMessage", { preview: logField(message, "preview") || t("config.remoteIm.logSendContentOmitted") }) };
  }
  if (message.startsWith("[联系人状态] 轮次结束:")) {
    const summary = stateSummary(message, t);
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindStatus"), title: summary, summary, detail: t("config.remoteIm.logTurnDetail", { decision: decisionLabel(logField(message, "decision"), t), followUp: boolLabel(logField(message, "follow_up"), t) }) };
  }
  if (message.startsWith("[联系人状态] 轮次收尾失败:")) {
    const summary = stateSummary(message, t);
    return { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindStatus"), title: summary, summary, detail: logField(message, "error") || undefined };
  }
  if (message.startsWith("[联系人状态] 异步发送收尾:")) return null;
  return log.level === "warn" || log.level === "error"
    ? { timestamp: log.timestamp, level: log.level, kind: t("config.remoteIm.logKindSystem"), title: t("config.remoteIm.logAbnormalTitle"), summary: t("config.remoteIm.logAbnormalSummary") }
    : null;
}
