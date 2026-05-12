<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

type ImpactItem = {
  path: string;
  adds: number;
  removes: number;
  kind: "update" | "add" | "delete" | "other";
};

const props = defineProps<{
  approvalKind?: string;
  command?: string;
  reviewOpinion?: string;
  impactSummary: ImpactItem[];
  patchKinds?: Array<"update" | "add" | "delete" | "other">;
}>();
const { t } = useI18n();

type RiskLevel = "high" | "medium" | "low" | "info";

const normalizedCommand = computed(() => String(props.command || "").trim());
const normalizedApprovalKind = computed(() => String(props.approvalKind || "").trim());
const normalizedPatchKinds = computed(() => Array.isArray(props.patchKinds) ? props.patchKinds : []);
const normalizedReviewOpinion = computed(() => String(props.reviewOpinion || "").trim());
const hasAiReviewSummary = computed(() =>
  normalizedApprovalKind.value.startsWith("ai_tool_review")
  && !!normalizedReviewOpinion.value,
);

const riskAssessment = computed(() => {
  const command = normalizedCommand.value.toLowerCase();
  const hasAddPatch = normalizedPatchKinds.value.includes("add");
  const hasDeletePatch = normalizedPatchKinds.value.includes("delete");
  const hasShellRedirection = />|>>/.test(command);
  const hasShellWriteIntent =
    hasShellRedirection
    || /\b(printf|echo|tee|touch|truncate)\b/.test(command)
    || /\b(set-content|add-content|out-file|new-item|copy-item|move-item|rename-item|remove-item)\b/.test(command);
  if (!command) {
    if (normalizedApprovalKind.value === "apply_patch_workspace_write") {
      const patchNotes = [
        hasAddPatch ? t("terminalApprovalImpact.patchAddNote") : "",
        hasDeletePatch ? t("terminalApprovalImpact.patchDeleteNote") : "",
      ].filter(Boolean).join(" ");
      return {
        level: "medium" as RiskLevel,
        label: t("terminalApprovalImpact.risk.medium"),
        reason: `${t("terminalApprovalImpact.workspaceWriteReason")}${patchNotes}`.trim(),
      };
    }
    if (normalizedApprovalKind.value === "read_file_preview") {
      return {
        level: "low" as RiskLevel,
        label: t("terminalApprovalImpact.risk.low"),
        reason: t("terminalApprovalImpact.readOnlyReason"),
      };
    }
    return {
      level: "info" as RiskLevel,
      label: t("terminalApprovalImpact.risk.info"),
      reason: t("terminalApprovalImpact.noCommandReason"),
    };
  }

  if (/\b(rm|del|erase|format|mkfs|dd|shutdown|reboot)\b/.test(command) || /\b(remove-item|reg delete|diskpart)\b/.test(command)) {
    return {
      level: "high" as RiskLevel,
      label: t("terminalApprovalImpact.risk.high"),
      reason: t("terminalApprovalImpact.destructiveReason"),
    };
  }

  if (hasShellWriteIntent || /\b(move|mv|copy|cp|xcopy|robocopy|mkdir|new-item|set-content|add-content|out-file|rename|ren)\b/.test(command) || /\b(git clean|git reset|git checkout)\b/.test(command)) {
    return {
      level: "medium" as RiskLevel,
      label: t("terminalApprovalImpact.risk.medium"),
      reason: t("terminalApprovalImpact.writeIntentReason"),
    };
  }

  if (/\b(cat|type|dir|ls|pwd|rg|findstr|git status|git diff|get-childitem)\b/.test(command)) {
    return {
      level: "low" as RiskLevel,
      label: t("terminalApprovalImpact.risk.low"),
      reason: t("terminalApprovalImpact.queryReason"),
    };
  }

  return {
    level: "info" as RiskLevel,
    label: t("terminalApprovalImpact.risk.info"),
    reason: t("terminalApprovalImpact.unknownReason"),
  };
});

const riskClassMap: Record<RiskLevel, string> = {
  high: "badge-error",
  medium: "badge-warning",
  low: "badge-success",
  info: "badge-ghost",
};

const impactKindLabelMap = computed<Record<ImpactItem["kind"], string>>(() => ({
  update: t("terminalApprovalImpact.kind.update"),
  add: t("terminalApprovalImpact.kind.add"),
  delete: t("terminalApprovalImpact.kind.delete"),
  other: t("terminalApprovalImpact.kind.other"),
}));

const impactKindClassMap: Record<ImpactItem["kind"], string> = {
  update: "badge-info",
  add: "badge-success",
  delete: "badge-error",
  other: "badge-ghost",
};
</script>

<template>
  <div class="mt-3 space-y-3">
    <div v-if="!hasAiReviewSummary" class="rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs font-medium text-base-content/60">{{ t("terminalApprovalImpact.riskTitle") }}</span>
        <span class="badge badge-sm" :class="riskClassMap[riskAssessment.level]">{{ riskAssessment.label }}</span>
      </div>
      <div class="mt-2 text-sm text-base-content/80">
        {{ riskAssessment.reason }}
      </div>
    </div>

    <div v-if="impactSummary.length > 0">
      <div>{{ t("terminalApprovalImpact.impactScope") }}</div>
      <div class="mt-2 space-y-2">
        <div
          v-for="item in impactSummary"
          :key="item.path"
          class="flex flex-wrap items-center gap-2 text-xs"
        >
          <span class="badge badge-sm" :class="impactKindClassMap[item.kind]">{{ impactKindLabelMap[item.kind] }}</span>
          <span class="font-mono text-base-content/80">{{ item.path }}</span>
          <span v-if="item.adds > 0" class="text-success">+{{ item.adds }}</span>
          <span v-if="item.removes > 0" class="text-error">-{{ item.removes }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
