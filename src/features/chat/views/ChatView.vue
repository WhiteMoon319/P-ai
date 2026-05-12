<template>
  <div
    ref="chatLayoutRoot"
    class="h-full min-h-0"
    :class="showSideConversationList && !detachedChatWindow ? 'flex flex-row overflow-hidden' : 'flex flex-col relative'"
  >
    <div
      v-if="showSideConversationList && !detachedChatWindow"
      class="flex h-full min-h-0 shrink-0"
      :style="{ width: `${leftSidebarWidth}px` }"
    >
      <ChatConversationSidebar
        :items="conversationItems || unarchivedConversationItems"
        :active-conversation-id="activeConversationId"
        :user-alias="userAlias"
        :user-avatar-url="userAvatarUrl"
        :persona-name-map="personaNameMap"
        :persona-avatar-url-map="personaAvatarUrlMap"
        @select="handleConversationListSelect"
        @rename="handleConversationRename"
        @toggle-pin-conversation="handleConversationPinToggle"
        @archive-conversation="handleConversationArchive"
        @delete-conversation="handleConversationDelete"
      />
    </div>

    <div
      v-if="showSideConversationList && !detachedChatWindow"
      class="ecall-pane-splitter ecall-pane-splitter-left"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'left' }"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.left.min"
      :aria-valuemax="PANE_WIDTH_LIMITS.left.max"
      :aria-valuenow="leftSidebarWidth"
      @pointerdown="startPaneResize('left', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('left', -24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('left', 24)"
    ></div>

    <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden">
      <div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
        <div
          v-if="mediaDragActive && !chatting && !frozen && !conversationBusy"
          class="pointer-events-none absolute inset-0 z-40 flex items-center justify-center bg-base-100/70 backdrop-blur-[1px]"
        >
          <div class="rounded-box border border-primary/40 bg-base-100 px-4 py-2 text-sm font-medium text-primary">
            {{ t("chat.dropImageOrPdf") }}
          </div>
        </div>

        <div class="relative flex min-h-0 flex-1 overflow-hidden" @mouseenter="chatScrollbarRef?.reveal()" @mouseleave="chatScrollbarRef?.hide()">
          <div
            ref="scrollContainer"
            class="ecall-chat-scroll-container relative flex flex-1 min-h-0 flex-col overflow-x-hidden overflow-y-auto px-0 py-3"
            :class="chatting || frozen || conversationBusy ? 'pointer-events-auto' : ''"
            :data-chat-interaction-locked="chatting || frozen || conversationBusy ? 'true' : undefined"
            @scroll="onConversationScroll"
          >
          <div
            v-if="loadingOlderHistory"
            class="pointer-events-none sticky top-0 z-10 flex justify-center pb-2"
          >
            <div class="badge badge-ghost gap-2 border-base-300 bg-base-100/90 px-3 py-3 shadow-sm">
              <span class="loading loading-spinner loading-xs"></span>
              <span>{{ t("chat.loadingOlderMessages") }}</span>
            </div>
          </div>

          <div v-if="hasActiveOrPendingTodo" class="pointer-events-none sticky top-0 z-20 flex justify-center pt-1">
            <div
              class="dropdown dropdown-bottom pointer-events-auto"
              :aria-label="t('config.task.fields.todo')"
              @click.stop
              @mousedown.stop
            >
              <label
                tabindex="0"
                class="btn btn-sm max-w-[min(88vw,30rem)] flex-nowrap justify-start gap-2 overflow-hidden rounded-full border-base-300 bg-base-300 text-base-content hover:border-base-300 hover:bg-base-200 normal-case"
              >
                <ListTodo class="h-4 w-4 shrink-0 opacity-70" />
                <span class="min-w-0 flex-1 truncate text-left">{{ activeConversationTodoDisplay }}</span>
                <span
                  v-if="normalizedConversationTodos.length > 1"
                  class="badge badge-ghost badge-sm shrink-0"
                >+{{ normalizedConversationTodos.length - 1 }}</span>
              </label>
              <div
                v-if="normalizedConversationTodos.length > 1"
                tabindex="0"
                class="dropdown-content card card-compact mt-2 w-max max-w-[min(88vw,30rem)] border border-base-300 bg-base-100 shadow-xl"
              >
                <div class="card-body p-3">
                  <ul class="flex flex-col gap-3">
                    <li
                      v-for="(item, index) in normalizedConversationTodos"
                      :key="`${item.status}-${index}-${item.content}`"
                      class="flex items-start gap-3"
                      :title="item.content"
                    >
                      <span
                        class="inline-flex h-7 min-w-7 shrink-0 items-center justify-center rounded-full text-sm font-semibold"
                        :class="todoIndexClass(item.status)"
                      >{{ index + 1 }}</span>
                      <span
                        class="min-w-0 wrap-break-word pt-0.5 text-sm leading-6"
                        :class="item.status === 'completed'
                          ? 'text-base-content/55 line-through'
                          : item.status === 'in_progress'
                            ? 'text-base-content font-semibold'
                            : 'text-base-content'"
                      >{{ item.content }}</span>
                    </li>
                  </ul>
                </div>
              </div>
            </div>
          </div>
          <div class="ecall-chat-history-flow flex min-w-0 flex-col">
            <div
              class="relative min-w-0 w-full"
              :style="{ height: `${totalVirtualSize}px` }"
            >
              <div
                v-for="entry in virtualEntries"
                :key="entry.item.id"
                :data-index="entry.row.index"
                :data-render-item-id="entry.item.id"
                :ref="(el) => measureVirtualRow(entry.item.id, el)"
                class="absolute left-0 top-0 w-full ecall-chat-virtual-item"
                :style="{ transform: `translateY(${entry.row.start}px)` }"
              >
                <div
                  v-if="entry.item.kind === 'compaction'"
                  class="mt-4 flex items-center gap-3 text-[11px] text-base-content/45"
                >
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <button
                    type="button"
                    class="btn btn-ghost btn-xs shrink-0 gap-1.5 px-2 text-base-content/60 hover:text-base-content"
                    :title="t('chat.viewSummary')"
                    @click="openConversationSummary(entry.item.block, $event)"
                  >
                    <History class="h-3.5 w-3.5" />
                    <span>{{ t("chat.viewSummary") }}</span>
                  </button>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>

                <div
                  v-else-if="entry.item.kind === 'plan_started'"
                  class="mt-4 flex items-center gap-3 text-[11px] text-base-content/45"
                >
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <span class="shrink-0 rounded-full border border-base-300/80 bg-base-100 px-3 py-1 text-base-content/55">{{ t("chat.planStartedDivider") }}</span>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>

                <div
                  v-else-if="entry.item.kind === 'time_divider'"
                  class="my-3 flex items-center gap-3 px-3 text-[11px] text-base-content/45"
                >
                  <div class="h-px flex-1 bg-base-300/70"></div>
                  <time
                    class="shrink-0 rounded-full border border-base-300/70 bg-base-100/90 px-3 py-1 text-base-content/55 shadow-sm"
                    :datetime="entry.item.createdAt"
                  >
                    {{ formatTimeDividerLabel(entry.item.createdAt) }}
                  </time>
                  <div class="h-px flex-1 bg-base-300/70"></div>
                </div>

                <div
                  v-else-if="entry.item.kind === 'message'"
                  v-memo="messageMemoKey(entry.item.block, entry.item.renderId, entry.item.blockIndex, entry.item.compactWithPrevious)"
                >
                  <div
                    class="ecall-elastic-item-shell"
                    :style="entry.item.id === latestOwnElasticItemId ? { minHeight: `${latestOwnElasticMinHeight}px` } : undefined"
                  >
                    <ChatMessageItem
                      :active-conversation-id="activeConversationId"
                      :block="entry.item.block"
                      :selection-key="entry.item.renderId"
                      :selection-mode-enabled="messageSelectionModeEnabled"
                      :selected="selectedMessageRenderIdSet.has(entry.item.renderId)"
                      :chatting="chatting"
                      :busy="conversationBusy"
                      :frozen="frozen"
                      :user-alias="userAlias"
                      :user-avatar-url="userAvatarUrl"
                      :persona-name-map="personaNameMap"
                      :persona-avatar-url-map="personaAvatarUrlMap"
                      :stream-tool-calls="visibleStreamToolCalls"
                      :markdown-is-dark="markdownIsDark"
                      :playing-audio-id="playingAudioId"
                      :active-turn-user="false"
                      :compact-with-previous="entry.item.compactWithPrevious"
                      :can-regenerate="canRegenerateBlock(entry.item.block, entry.item.blockIndex)"
                      :can-confirm-plan="canConfirmPlan(entry.item.block)"
                      :bubble-background-hidden="isBubbleBackgroundHidden(entry.item.block)"
                      :hide-toggle-enabled="canToggleBubbleBackground(entry.item.block)"
                      @recall-turn="$emit('recallTurn', $event)"
                      @regenerate-turn="$emit('regenerateTurn', $event)"
                      @confirm-plan="$emit('confirmPlan', $event)"
                      @enter-selection-mode="enterMessageSelectionMode"
                      @toggle-message-selected="toggleMessageSelected"
                      @copy-message="copyMessage"
                      @open-image-preview="openImagePreview"
                      @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                      @assistant-link-click="handleAssistantLinkClick"
                      @toggle-bubble-background="toggleBubbleBackground(entry.item.block)"
                    />
                  </div>
                </div>

                <div
                  v-else
                  class="ecall-turn-group"
                >
                  <div
                    class="ecall-turn-stack"
                    :style="entry.item.id === latestOwnElasticItemId ? { minHeight: `${latestOwnElasticMinHeight}px` } : undefined"
                  >
                    <template v-for="groupItem in entry.item.items" :key="groupItem.renderId">
                      <ChatMessageItem
                        v-memo="messageMemoKey(groupItem.block, groupItem.renderId, groupItem.blockIndex, groupItem.compactWithPrevious)"
                        :active-conversation-id="activeConversationId"
                        :block="groupItem.block"
                        :selection-key="groupItem.renderId"
                        :selection-mode-enabled="messageSelectionModeEnabled"
                        :selected="selectedMessageRenderIdSet.has(groupItem.renderId)"
                        :chatting="chatting"
                        :busy="conversationBusy"
                        :frozen="frozen"
                        :user-alias="userAlias"
                        :user-avatar-url="userAvatarUrl"
                        :persona-name-map="personaNameMap"
                        :persona-avatar-url-map="personaAvatarUrlMap"
                        :stream-tool-calls="visibleStreamToolCalls"
                        :markdown-is-dark="markdownIsDark"
                        :playing-audio-id="playingAudioId"
                        :active-turn-user="false"
                        :compact-with-previous="groupItem.compactWithPrevious"
                        :can-regenerate="canRegenerateBlock(groupItem.block, groupItem.blockIndex)"
                        :can-confirm-plan="canConfirmPlan(groupItem.block)"
                        :bubble-background-hidden="isBubbleBackgroundHidden(groupItem.block)"
                        :hide-toggle-enabled="canToggleBubbleBackground(groupItem.block)"
                        @recall-turn="$emit('recallTurn', $event)"
                        @regenerate-turn="$emit('regenerateTurn', $event)"
                        @confirm-plan="$emit('confirmPlan', $event)"
                        @enter-selection-mode="enterMessageSelectionMode"
                        @toggle-message-selected="toggleMessageSelected"
                        @copy-message="copyMessage"
                        @open-image-preview="openImagePreview"
                        @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                        @assistant-link-click="handleAssistantLinkClick"
                        @toggle-bubble-background="toggleBubbleBackground(groupItem.block)"
                      />
                    </template>
                  </div>
                </div>
              </div>
            </div>

            <div ref="toolbarContainer" class="ecall-chat-toolbar-shell px-2 pt-1 pb-2">
              <ChatWorkspaceToolbar
                :chatting="chatting"
                :frozen="frozen"
                :conversation-busy="conversationBusy"
                :workspace-button-label="t('chat.allowedWorkspaceButton')"
                :workspace-button-name="currentWorkspaceName"
                :workspace-button-disabled="!activeConversationId || activeConversationSummary?.kind === 'remote_im_contact'"
                :hide-menu-button="activeConversationSummary?.kind === 'remote_im_contact'"
                :hide-workspace-button="activeConversationSummary?.kind === 'remote_im_contact'"
                :mention-entries="mentionEntries"
                :selected-mention-keys="selectedMentionKeys"
                :supervision-active="supervisionActive"
                :supervision-label="t('chat.supervision.button')"
                :supervision-active-label="t('chat.supervision.buttonActive')"
                :supervision-title="supervisionButtonTitle"
                :supervision-disabled="activeConversationSummary?.kind === 'remote_im_contact'"
                :review-button-label="toolReviewButtonLabel"
                :review-button-count="toolReviewButtonCount"
                :review-panel-open="toolReviewPanelOpen"
                :review-button-enabled="toolReviewButtonEnabled"
                :show-detach-button="!detachedChatWindow && !activeConversationSummary?.isMainConversation"
                :detach-disabled="!activeConversationId || activeConversationSummary?.isMainConversation || chatting || frozen || conversationBusy"
                @lock-workspace="$emit('lockWorkspace')"
                @open-branch-selection="openBranchSelectionMenu"
                @open-delegate-selection="openDelegateSelectionMenu"
                @open-forward-selection="openForwardSelectionMenu"
                @open-share-selection="openShareSelectionMenu"
                @mention-entry="entry => {
                  const agentId = String(entry?.agentId || '').trim();
                  const departmentId = String(entry?.departmentId || '').trim();
                  if (!agentId || !departmentId) return;
                  const mentionKey = `${agentId}:${departmentId}`;
                  if (selectedMentionKeys.includes(mentionKey)) {
                    emit('removeMention', { agentId, departmentId });
                    return;
                  }
                  emit('addMention', {
                    agentId,
                    agentName: String(entry?.agentName || '').trim() || agentId,
                    departmentId,
                    departmentName: String(entry?.departmentName || '').trim() || departmentId,
                    avatarUrl: String(entry?.avatarUrl || '').trim() || undefined,
                  });
                }"
                @open-supervision-task="$emit('openSupervisionTask')"
                @detach-conversation="handleDetachConversationRequest"
                @toggle-tool-review="toggleToolReviewPanel"
              />
            </div>
          </div>
          </div>
          <FloatingScrollbar ref="chatScrollbarRef" :target="scrollContainer" />
        </div>
        <div
          v-if="conversationSummaryCard.visible"
          class="pointer-events-none absolute inset-x-0 top-4 z-30 flex justify-center px-3"
        >
          <div class="pointer-events-auto w-full max-w-xl rounded-box border border-base-300 bg-base-100 shadow-xl">
            <div class="flex items-center justify-between gap-3 border-b border-base-300 px-4 py-3">
              <div class="text-sm font-semibold">{{ t("chat.viewSummary") }}</div>
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                @click="closeConversationSummaryCard"
              >
                {{ t("common.close") }}
              </button>
            </div>
            <div class="max-h-[min(52vh,28rem)] overflow-y-auto px-4 py-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-6 text-base-content/85">
                {{ conversationSummaryCard.text }}
              </div>
            </div>
          </div>
        </div>
        <div
          v-show="showJumpToBottom"
          class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end"
          :style="jumpToBottomStyle"
        >
          <button
            class="btn btn-sm btn-circle btn-primary pointer-events-auto shadow-lg"
            :title="t('chat.jumpToBottom')"
            @click="handleJumpToBottom"
          >
            <ChevronsDown class="h-4 w-4" />
          </button>
        </div>

        <div ref="composerContainer" class="relative shrink-0 border-t border-base-300 bg-base-100 p-2">
          <div
            v-if="chatStatusBanner"
            class="pointer-events-none absolute inset-x-0 top-0 z-10 -translate-y-full"
          >
            <div
              class="relative flex w-full items-center justify-center rounded-none px-4 py-1.5 text-center text-[12px] backdrop-blur-md"
              :class="chatStatusBanner.tone === 'error'
                ? 'bg-error/12 text-error'
                : chatStatusBanner.text === t('chat.statusCompactingContext')
                  ? 'bg-info/12 text-info'
                  : 'bg-base-200/75 text-base-content'"
            >
              <span
                class="relative z-1"
                :class="chatStatusBanner.tone === 'error'
                  ? ''
                  : 'text-base-content/80 ecall-shimmer-text ecall-reasoning-shimmer'"
                :data-shimmer-text="chatStatusBanner.tone === 'error' ? '' : chatStatusBanner.text"
              >{{ chatStatusBanner.text }}</span>
            </div>
          </div>
          <ChatApprovalPanel
            v-if="activeConversationTerminalApprovals.length > 0"
            :approvals="activeConversationTerminalApprovals"
            :resolving="terminalApprovalResolving"
            @approve="$emit('approveTerminalApproval', $event)"
            @deny="$emit('denyTerminalApproval', $event)"
          />
          <ChatComposerPanel
            v-else
            ref="composerPanelRef"
            :selection-mode-enabled="messageSelectionModeEnabled"
            :selected-message-count="selectedMessageBlocks.length"
            :chat-input="chatInput"
            :instruction-presets="instructionPresets"
            :mention-entries="mentionEntries"
            :selected-mentions="selectedMentions"
            :chat-input-placeholder="chatInputPlaceholder"
            :clipboard-images="clipboardImages"
            :queued-attachment-notices="queuedAttachmentNotices"
            :link-open-error-text="linkOpenErrorText"
            :chat-error-text="chatErrorText"
            :transcribing="transcribing"
            :can-record="canRecord"
            :recording="recording"
            :recording-ms="recordingMs"
            :record-hotkey="recordHotkey"
            :selected-chat-model-id="selectedChatModelId"
            :chat-model-options="chatModelOptions"
            :plan-mode-enabled="planModeEnabled"
            :frontend-round-phase="frontendRoundPhase"
            :chat-usage-percent="chatUsagePercent"
            :force-archive-tip="forceArchiveTip"
            :chatting="chatting"
            :busy="conversationBusy"
            :stop-chat-disabled="isOrganizingContextBusy"
            :frozen="frozen"
            :show-side-conversation-list="detachedChatWindow ? false : showSideConversationList"
            :active-conversation-id="activeConversationId"
            :unarchived-conversation-items="unarchivedConversationItems"
            :user-alias="userAlias"
            :user-avatar-url="userAvatarUrl"
            :persona-name-map="personaNameMap"
            :persona-avatar-url-map="personaAvatarUrlMap"
            :create-conversation-department-options="createConversationDepartmentOptions"
            :delegate-department-ids="delegateDepartmentIds"
            :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
            :ide-context-groups="visibleIdeContextGroups"
            :attached-ide-context-references="attachedIdeContextReferences"
            @update:chat-input="$emit('update:chatInput', $event)"
            @add-mention="$emit('addMention', $event)"
            @remove-mention="$emit('removeMention', $event)"
            @remove-clipboard-image="$emit('removeClipboardImage', $event)"
            @remove-queued-attachment-notice="$emit('removeQueuedAttachmentNotice', $event)"
            @start-recording="$emit('startRecording')"
            @stop-recording="$emit('stopRecording')"
            @pick-attachments="$emit('pickAttachments')"
            @update:selected-chat-model-id="$emit('update:selectedChatModelId', $event)"
            @update:plan-mode-enabled="$emit('update:planModeEnabled', $event)"
            @attach-ide-context-reference="handleAttachIdeContextReference"
            @remove-ide-context-reference="handleRemoveIdeContextReference"
            @send-chat="handleSendChat"
            @stop-chat="$emit('stopChat')"
            @exit-selection-mode="exitMessageSelectionMode"
            @selection-action-copy="copySelectedMessages"
            @selection-action-branch="emitSelectionAction('branch')"
            @selection-action-forward="emitSelectionAction('forward', $event)"
            @selection-action-delegate="emitSelectionAction('delegate', $event)"
            @selection-action-share="emitSelectionAction('share', $event)"
            @force-archive="$emit('forceArchive')"
            @switch-conversation="$emit('switchConversation', $event)"
            @create-conversation="$emit('createConversation', $event)"
          />
        </div>

        <ChatImagePreviewDialog
          :open="imagePreviewOpen"
          :data-url="imagePreviewDataUrl"
          :zoom="imagePreviewZoom"
          :min-zoom="IMAGE_PREVIEW_MIN_ZOOM"
          :max-zoom="IMAGE_PREVIEW_MAX_ZOOM"
          :offset-x="previewOffsetX"
          :offset-y="previewOffsetY"
          :dragging="previewDragging"
          @close="closeImagePreview"
          @zoom-in="zoomInPreview"
          @zoom-out="zoomOutPreview"
          @reset="resetPreviewZoom"
          @wheel="onPreviewWheel"
          @pointer-down="onPreviewPointerDown"
          @pointer-move="onPreviewPointerMove"
          @pointer-up="onPreviewPointerUp"
        />

        <ChatSupervisionTaskDialog
          :open="supervisionDialogOpen"
          :saving="supervisionTaskSaving"
          :error-text="supervisionTaskError"
          :active-task="activeSupervisionTask"
          :recent-history="recentSupervisionTaskHistory"
          @close="$emit('closeSupervisionTask')"
          @save="$emit('saveSupervisionTask', $event)"
        />
      </div>

      <div
        v-if="toolReviewPanelOpen"
        class="ecall-pane-splitter ecall-pane-splitter-right"
        :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'right' }"
        role="separator"
        tabindex="0"
        aria-orientation="vertical"
        :aria-valuemin="PANE_WIDTH_LIMITS.right.min"
        :aria-valuemax="PANE_WIDTH_LIMITS.right.max"
        :aria-valuenow="rightSidebarWidth"
        @pointerdown="startPaneResize('right', $event)"
        @keydown.left.prevent="adjustPaneWidthByKeyboard('right', 24)"
        @keydown.right.prevent="adjustPaneWidthByKeyboard('right', -24)"
      ></div>

      <div
        v-if="toolReviewPanelOpen"
        class="flex h-full min-h-0 shrink-0 border-l border-base-300 bg-base-100 pt-2"
        :style="{ width: `${rightSidebarWidth}px` }"
      >
        <ToolReviewSidebar
          ref="toolReviewSidebarRef"
          class="w-full"
          :batches="toolReviewBatches"
          :current-batch-key="toolReviewCurrentBatchKey"
          :detail-map="toolReviewDetailMap"
          :detail-loading-call-id="toolReviewDetailLoadingCallId"
          :reviewing-call-id="toolReviewReviewingCallId"
          :batch-reviewing-key="toolReviewBatchReviewingKey"
          :submitting-batch-key="toolReviewSubmittingBatchKey"
          :error-text="toolReviewErrorText"
          :report-error-text="toolReviewReportErrorText"
          :reports="toolReviewReports"
          :current-report-id="toolReviewCurrentReportId"
          :markdown-is-dark="markdownIsDark"
          :current-workspace-name="currentWorkspaceName"
          :current-workspace-root-path="currentWorkspaceRootPath"
          :workspaces="workspaces"
          :current-department-id="currentDepartmentId"
          :department-options="toolReviewDepartmentOptions"
          :delegate-statuses="delegateStatuses"
          :delegate-loading="delegateStatusesLoading"
          :delegate-error-text="delegateStatusesErrorText"
          @select-batch="setToolReviewCurrentBatchKey"
          @load-item-detail="loadToolReviewItemDetail"
          @review-item="runToolReviewForCall"
          @review-batch="runToolReviewForBatch"
          @pick-commit-review="handlePickCommitReview"
          @review-code="handleToolReviewCode"
          @retry-report="handleRetryToolReviewReport"
          @delete-report="handleDeleteToolReviewReport"
          @copy-report="copyToolReviewReport"
          @attach-report="$emit('attachToolReviewReport', $event)"
          @open-delegate-detail="openDelegateArchiveDetail"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch, type ComponentPublicInstance } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { useI18n } from "vue-i18n";
import { isDarkAppTheme } from "../../shell/composables/use-app-theme";
import { ChevronsDown, History, ListTodo } from "lucide-vue-next";
import "markstream-vue/index.css";
import { invokeTauri } from "../../../services/tauri-api";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, ChatMessageBlock, ChatPersonaPresenceChip, ChatTodoItem, ConversationDelegateStatusSummary, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset, ShellWorkspace } from "../../../types/app";
import ChatMessageItem from "../components/ChatMessageItem.vue";
import ChatApprovalPanel from "../components/ChatApprovalPanel.vue";
import ChatComposerPanel from "../components/ChatComposerPanel.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import ChatConversationSidebar from "../components/ChatConversationSidebar.vue";
import ChatWorkspaceToolbar from "../components/ChatWorkspaceToolbar.vue";
import ToolReviewSidebar from "../components/ToolReviewSidebar.vue";
import ChatImagePreviewDialog from "../components/dialogs/ChatImagePreviewDialog.vue";
import ChatSupervisionTaskDialog from "../components/dialogs/ChatSupervisionTaskDialog.vue";
import { useChatImagePreview } from "../composables/use-chat-image-preview";
import { useChatMessageActions } from "../composables/use-chat-message-actions";
import { useChatScrollLayout } from "../composables/use-chat-scroll-layout";
import { useChatToolReview, type ToolReviewCodeReviewScope, type ToolReviewCommitOption, type ToolReviewReportRecord } from "../composables/use-chat-tool-review";
import { resolveRetryToolReviewDepartmentId } from "../utils/tool-review-department";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import { isAbsoluteLocalPath, normalizeLocalLinkHref } from "../utils/local-link";
import { type ChatRenderItem, isRightAlignedMessage } from "../utils/chat-render";
import { canOpenInFileReader } from "../utils/chat-render";
import { useIdeContext } from "../composables/use-ide-context";
import { useDelegateStatus } from "../composables/use-delegate-status";
import { useBubbleBackground } from "../composables/use-bubble-background";
import { useChatVirtualList, blockRenderId, blockGroupRenderId, estimateChatRenderItemHeight } from "../composables/use-chat-virtual-list";


const props = defineProps<{
  userAlias: string;
  personaName: string;
  userAvatarUrl: string;
  assistantAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  mentionEntries: ChatMentionEntry[];
  selectedMentions: ChatMentionTarget[];
  latestUserText: string;
  latestUserImages: Array<{ mime: string; bytesBase64: string }>;
  latestAssistantText: string;
  latestReasoningStandardText: string;
  latestReasoningInlineText: string;
  frontendRoundPhase: "idle" | "queued" | "waiting" | "streaming";
  toolStatusText: string;
  toolStatusState: "running" | "done" | "failed" | "";
  streamToolCalls: Array<{ name: string; argsText: string; status?: "doing" | "done" }>;
  chatErrorText: string;
  clipboardImages: Array<{ mime: string; bytesBase64: string }>;
  queuedAttachmentNotices: Array<{ id: string; fileName: string; relativePath: string; mime: string }>;
  chatInput: string;
  instructionPresets: PromptCommandPreset[];
  chatInputPlaceholder: string;
  canRecord: boolean;
  recording: boolean;
  recordingMs: number;
  transcribing: boolean;
  recordHotkey: string;
  selectedChatModelId: string;
  toolReviewRefreshTick: number;
  chatModelOptions: ApiConfigItem[];
  planModeEnabled: boolean;
  chatUsagePercent: number;
  forceArchiveTip: string;
  mediaDragActive: boolean;
  chatting: boolean;
  forcingArchive: boolean;
  forcingArchiveConversationId?: string;
  compactingConversation: boolean;
  compactingConversationId?: string;
  conversationBusy: boolean;
  frozen: boolean;
  messageBlocks: ChatMessageBlock[];
  hasMoreHistory: boolean;
  loadingOlderHistory: boolean;
  latestOwnMessageAlignRequest: number;
  conversationScrollToBottomRequest: number;
  currentWorkspaceName: string;
  currentWorkspaceRootPath: string;
  workspaces: ShellWorkspace[];
  currentDepartmentId: string;
  activeConversationId: string;
  currentTodos: ChatTodoItem[];
  supervisionActive: boolean;
  supervisionTitle: string;
  supervisionDialogOpen: boolean;
  supervisionTaskSaving: boolean;
  supervisionTaskError: string;
  activeSupervisionTask: {
    taskId: string;
    goal: string;
    why: string;
    todo: string;
    endAtLocal: string;
    remainingHours: number;
  } | null;
  recentSupervisionTaskHistory: Array<{
    goal: string;
    why: string;
    todo: string;
    durationHours: number;
  }>;
  currentTheme: string;
  unarchivedConversationItems: ChatConversationOverviewItem[];
  conversationItems?: ChatConversationOverviewItem[];
  sideConversationListVisible: boolean;
  createConversationDepartmentOptions: Array<{ id: string; name: string; ownerAgentId?: string; ownerName: string; providerName?: string; modelName?: string }>;
  delegateDepartmentIds: string[];
  defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[];
  attachedIdeContextReferences: IdeContextReferenceItem[];
  detachedChatWindow?: boolean;
  terminalApprovals?: TerminalApprovalConversationItem[];
  terminalApprovalResolving?: boolean;
}>();

const markdownIsDark = computed(() => isDarkAppTheme(props.currentTheme));
const toolReviewDepartmentOptions = computed(() => {
  const allowedDepartmentIds = new Set(
    (Array.isArray(props.delegateDepartmentIds) ? props.delegateDepartmentIds : [])
      .map((id) => String(id || "").trim())
      .filter(Boolean),
  );
  return (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : [])
    .filter((item) => allowedDepartmentIds.has(String(item.id || "").trim()));
});
const activeConversationSummary = computed(() => {
  const activeConversationId = String(props.activeConversationId || "").trim();
  if (!activeConversationId) return null;
  const matched = (props.conversationItems || props.unarchivedConversationItems).find(
    (item) => String(item.conversationId || "").trim() === activeConversationId,
  ) || null;
  return matched;
});
const {
  visibleIdeContextGroups,
  attachedIdeContextReferences,
  attachReference: handleAttachIdeContextReference,
  removeReference: handleRemoveIdeContextReference,
  clearAttachedReferences: clearAttachedIdeContextReferences,
} = useIdeContext({
  activeConversationId: toRef(props, "activeConversationId"),
  workspaces: toRef(props, "workspaces"),
  currentWorkspaceRootPath: toRef(props, "currentWorkspaceRootPath"),
  currentWorkspaceName: toRef(props, "currentWorkspaceName"),
});
function isOrganizeContextToolCall(call: { name: string; argsText: string; status?: "doing" | "done" }): boolean {
  const name = String(call.name || "").trim().toLowerCase();
  if (name === "organize_context" || name === "archive") return true;
  return false;
}



async function handlePickCommitReview(page = 1) {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return;
  toolReviewSidebarRef.value?.setCommitOptions([], true, 0, page, 30);
  try {
    const result = await listToolReviewCommitOptions(conversationId, page, 30);
    toolReviewSidebarRef.value?.setCommitOptions(result.commits, false, result.total, result.page, result.pageSize);
  } catch (error) {
    const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
    toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || "Unknown error" });
    toolReviewSidebarRef.value?.setCommitOptions([], false, 0, page, 30);
  }
}

async function handleDeleteToolReviewReport(report: ToolReviewReportRecord) {
  const conversationId = String(props.activeConversationId || "").trim();
  const reportId = String(report.id || "").trim();
  if (!conversationId || !reportId) return;
  try {
    await deleteToolReviewReport({
      conversationId,
      reportId,
    });
  } catch (error) {
    const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
    console.error("[工具审查][前端] 删除审查报告失败", {
      conversationId,
      reportId,
      error,
    });
    toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || t("chat.codeReviewDeleteFailed") });
  }
}

function handleToolReviewCode(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }) {
  const conversationId = String(props.activeConversationId || "").trim();
  const scope = input.scope;
  const normalizedTarget = String(input.target || "").trim();
  const departmentId = String(input.departmentId || props.currentDepartmentId || "").trim();
  if (!conversationId) {
    toolReviewErrorText.value = t("chat.codeReviewNoConversation");
    return;
  }
  if (!departmentId) {
    toolReviewErrorText.value = t("chat.codeReviewNoDepartment");
    return;
  }
  console.info("[工具审查][前端] 发起代码审查任务", {
    conversationId,
    scope,
    target: normalizedTarget,
    departmentId,
  });
  void submitToolReviewCode({
    conversationId,
    scope,
    target: normalizedTarget || undefined,
    departmentId,
  });
}

async function handleRetryToolReviewReport(report: ToolReviewReportRecord) {
  const scope = String(report.scope || "").trim() as ToolReviewCodeReviewScope;
  const target = String(report.target || "").trim();
  const conversationId = String(props.activeConversationId || "").trim();
  const reportId = String(report.id || "").trim();
  if (!conversationId || !reportId) {
    toolReviewErrorText.value = t("chat.codeReviewRetryNoConversation");
    return;
  }
  const retryCodeReviewDepartmentId = resolveRetryToolReviewDepartmentId({
    reportDepartmentId: String(report.departmentId || "").trim(),
    currentDepartmentId: String(props.currentDepartmentId || "").trim(),
    departmentOptions: toolReviewDepartmentOptions.value,
  });
  if (scope === "commit" || scope === "main" || scope === "uncommitted" || scope === "custom") {
    if (!retryCodeReviewDepartmentId) {
      toolReviewErrorText.value = t("chat.codeReviewRetryNoDepartment");
      return;
    }
    console.info("[工具审查][前端] 重新生成代码审查", {
      conversationId,
      reportId,
      scope,
      target,
      departmentId: retryCodeReviewDepartmentId,
    });
    const nextReport = await submitToolReviewCode({
      conversationId,
      scope,
      target: target || undefined,
      departmentId: retryCodeReviewDepartmentId,
    });
    if (!nextReport) return;
    try {
      await deleteToolReviewReport({
        conversationId,
        reportId,
      });
    } catch (error) {
      const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
      console.error("[工具审查][前端] 删除旧代码审查报告失败", {
        conversationId,
        reportId,
        scope,
        target,
        error,
      });
      toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || "删除旧代码审查报告失败" });
    }
    return;
  }
  toolReviewErrorText.value = t("chat.codeReviewRetryUnsupportedScope", { scope: scope || "" });
}

function handleViewApprovalDetail() {
  if (!toolReviewPanelOpen.value) {
    toggleToolReviewPanel();
  }
}

const visibleStreamToolCalls = computed(() =>
  props.streamToolCalls.filter((call) => !isOrganizeContextToolCall(call))
);

const normalizedConversationTodos = computed(() => {
  const todos = Array.isArray(props.currentTodos) ? props.currentTodos : [];
  return todos
    .map((item) => ({
      content: String(item?.content || "").trim(),
      status: String(item?.status || "").trim() as ChatTodoItem["status"],
    }))
    .filter((item) => item.content && (item.status === "pending" || item.status === "in_progress" || item.status === "completed"));
});

const hasActiveOrPendingTodo = computed(() => {
  return normalizedConversationTodos.value.some(item => item.status === "pending" || item.status === "in_progress");
});

const isCurrentConversationCompacting = computed(() => {
  const currentConversationId = String(props.activeConversationId || "").trim();
  const compactingConversationId = String(props.compactingConversationId || "").trim();
  return !!currentConversationId && currentConversationId === compactingConversationId;
});

const isCurrentConversationArchiving = computed(() => {
  const currentConversationId = String(props.activeConversationId || "").trim();
  const forcingArchiveConversationId = String(props.forcingArchiveConversationId || "").trim();
  return !!currentConversationId && currentConversationId === forcingArchiveConversationId;
});

const activeConversationTodoIndex = computed(() => {
  const todos = normalizedConversationTodos.value;
  const inProgressIndex = todos.findIndex((item) => item.status === "in_progress");
  if (inProgressIndex >= 0) return inProgressIndex;
  const pendingIndex = todos.findIndex((item) => item.status === "pending");
  if (pendingIndex >= 0) return pendingIndex;
  return todos.length ? 0 : -1;
});

const activeConversationTodo = computed(() => {
  const index = activeConversationTodoIndex.value;
  if (index < 0) return "";
  return String(normalizedConversationTodos.value[index]?.content || "").trim();
});

const activeConversationTodoDisplay = computed(() => {
  const todo = activeConversationTodo.value;
  if (!todo) return "";
  const personaName = String(props.personaName || "").trim();
  return personaName
    ? t("chat.todoIntentionWithPersona", { name: personaName, todo })
    : t("chat.todoIntention", { todo });
});

const activeConversationTerminalApprovals = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return [];
  const approvals = Array.isArray(props.terminalApprovals) ? props.terminalApprovals : [];
  return approvals.filter((item) => String(item.conversationId || item.sessionId || "").trim() === conversationId);
});

const supervisionButtonTitle = computed(() => {
  const baseTitle = props.supervisionActive
    ? t("chat.supervision.activeButtonTitle")
    : t("chat.supervision.buttonTitle");
  const detail = String(props.supervisionTitle || "").trim();
  return detail ? `${baseTitle}\n${detail}` : baseTitle;
});

function todoIndexClass(status: ChatTodoItem["status"]): string {
  if (status === "completed") return "bg-success text-success-content";
  if (status === "in_progress") return "bg-primary text-primary-content";
  return "bg-base-200 text-base-content/70";
}

const activeRunningToolCall = computed(() => {
  if (props.toolStatusState !== "running") return null;
  const calls = Array.isArray(props.streamToolCalls) ? props.streamToolCalls : [];
  for (let idx = calls.length - 1; idx >= 0; idx -= 1) {
    const call = calls[idx];
    if (String(call?.status || "").trim() === "done") continue;
    const name = String(call?.name || "").trim();
    if (!name) continue;
    return call;
  }
  return null;
});

const isOrganizingContextBusy = computed(() => {
  if (props.compactingConversation && isCurrentConversationCompacting.value) return true;
  const runningTool = activeRunningToolCall.value;
  if (runningTool && isOrganizeContextToolCall(runningTool)) return true;
  const statusText = String(props.toolStatusText || "").trim();
  return props.toolStatusState === "running" && (
    statusText.includes("整理上下文") || statusText.includes("自动整理")
  );
});

const chatStatusBanner = computed<null | { text: string; tone: "default" | "error" }>(() => {
  const errorText = String(props.chatErrorText || "").trim();
  if (errorText) {
    return {
      text: errorText,
      tone: "error",
    };
  }
  if (props.forcingArchive) {
    if (!isCurrentConversationArchiving.value) return null;
    return {
      text: t("chat.statusArchivingConversation"),
      tone: "default",
    };
  }
  if (isOrganizingContextBusy.value) {
    return {
      text: t("chat.statusCompactingContext"),
      tone: "default",
    };
  }
  return null;
});

const selectedMentionKeys = computed(() =>
  (Array.isArray(props.selectedMentions) ? props.selectedMentions : [])
    .map((item) => {
      const agentId = String(item?.agentId || "").trim();
      const departmentId = String(item?.departmentId || "").trim();
      return agentId && departmentId ? `${agentId}:${departmentId}` : "";
    })
    .filter((value, index, list) => !!value && list.indexOf(value) === index),
);
const latestPendingPlanMessageId = computed(() => {
  for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
    const block = props.messageBlocks[idx];
    if (block.isExtraTextBlock) continue;
    const providerMeta = (block.providerMeta || {}) as Record<string, unknown>;
    const messageMeta = ((providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>);
    const messageKind = String(messageMeta.kind || providerMeta.messageKind || "").trim();
    if (messageKind === "plan_complete" || block.planCard?.action === "complete") {
      return "";
    }
    if (messageKind === "plan_present" || block.planCard?.action === "present") {
      return String(block.sourceMessageId || block.id || "").trim();
    }
  }
  return "";
});

const emit = defineEmits<{
  (e: "update:chatInput", value: string): void;
  (e: "update:selectedInstructionPrompts", value: PromptCommandPreset[]): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "sideConversationListVisibleChange", value: boolean): void;
  (e: "toolReviewPanelOpenChange", value: boolean): void;
  (e: "sidePanelWidthsChange", value: { leftWidth: number; rightWidth: number }): void;
  (e: "sidePanelWidthsCommit", value: { leftWidth: number; rightWidth: number }): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void;
  (e: "stopRecording"): void;
  (e: "pickAttachments"): void;
  (e: "update:selectedChatModelId", value: string): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "sendChat", payload?: { extraTextBlocks?: string[] }): void;
  (e: "stopChat"): void;
  (e: "forceArchive"): void;
  (e: "recallTurn", payload: { turnId: string }): void;
  (e: "regenerateTurn", payload: { turnId: string }): void;
  (e: "confirmPlan", payload: { messageId: string }): void;
  (e: "lockWorkspace"): void;
  (e: "openSupervisionTask"): void;
  (e: "detachConversation"): void;
  (e: "closeSupervisionTask"): void;
  (e: "saveSupervisionTask", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
  (e: "switchConversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "renameConversation", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
  (e: "createConversation", input?: { title?: string; departmentId?: string }): void;
  (e: "loadOlderHistory"): void;
  (e: "reachedBottom"): void;
  (e: "jumpToConversationBottom"): void;
  (e: "refreshToolReviewMessage", payload: { conversationId: string; messageId: string }): void;
  (e: "attachToolReviewReport", reportText: string): void;
  (e: "selectionActionCopy", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionCopyError", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; error: string }): void;
  (e: "selectionActionBranch", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[] }): void;
  (e: "selectionActionForward", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; targetConversationId: string }): void;
  (e: "selectionActionDelegate", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; departmentId: string; presetId: string; background: string; question: string; focus: string }): void;
  (e: "selectionActionShare", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; exportFormat?: "html" | "png" }): void;
  (e: "approveTerminalApproval", requestId: string): void;
  (e: "denyTerminalApproval", requestId: string): void;
}>();
const { t } = useI18n();
const toolReviewSidebarRef = ref<ComponentPublicInstance<{ setCommitOptions: (items: ToolReviewCommitOption[], loading?: boolean, total?: number, page?: number, pageSize?: number) => void }> | null>(null);
const chatScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);

type PaneResizeSide = "left" | "right";

const PANE_WIDTH_LIMITS = {
  left: { min: 240, max: 560, default: 320 },
  right: { min: 280, max: 680, default: 320 },
} as const;
const PANE_CENTER_MIN_WIDTH = 360;
const PANE_WIDTH_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-width",
  right: "easy-call.chat.right-sidebar-width",
} as const;

const leftSidebarWidth = ref(loadStoredPaneWidth("left"));
const rightSidebarWidth = ref(loadStoredPaneWidth("right"));
const activePaneResizeSide = ref<PaneResizeSide | null>(null);
let paneResizeStartX = 0;
let paneResizeStartWidth = 0;
let paneResizePreviousBodyCursor = "";
let paneResizePreviousBodyUserSelect = "";

function loadStoredPaneWidth(side: PaneResizeSide): number {
  if (typeof window === "undefined") return PANE_WIDTH_LIMITS[side].default;
  const rawValue = window.localStorage.getItem(PANE_WIDTH_STORAGE_KEYS[side]);
  const parsedValue = Number(rawValue);
  if (!Number.isFinite(parsedValue)) return PANE_WIDTH_LIMITS[side].default;
  const limits = PANE_WIDTH_LIMITS[side];
  return Math.round(Math.min(limits.max, Math.max(limits.min, parsedValue)));
}

function storePaneWidth(side: PaneResizeSide, width: number) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(PANE_WIDTH_STORAGE_KEYS[side], String(Math.round(width)));
}

function clampPaneWidth(side: PaneResizeSide, width: number): number {
  const limits = PANE_WIDTH_LIMITS[side];
  const layoutWidth = chatLayoutRoot.value?.getBoundingClientRect().width || 0;
  const otherPaneWidth =
    side === "left"
      ? (toolReviewPanelOpen.value ? rightSidebarWidth.value : 0)
      : (showSideConversationList.value && !props.detachedChatWindow ? leftSidebarWidth.value : 0);
  const layoutMax = layoutWidth > 0 ? layoutWidth - otherPaneWidth - PANE_CENTER_MIN_WIDTH : limits.max;
  const effectiveMax = Math.max(limits.min, Math.min(limits.max, layoutMax));
  return Math.round(Math.min(effectiveMax, Math.max(limits.min, width)));
}

function setPaneWidth(side: PaneResizeSide, width: number, persist = false) {
  const nextWidth = clampPaneWidth(side, width);
  if (side === "left") {
    leftSidebarWidth.value = nextWidth;
  } else {
    rightSidebarWidth.value = nextWidth;
  }
  if (persist) {
    storePaneWidth(side, nextWidth);
  }
  void nextTick(() => syncViewportMetrics());
}

function startPaneResize(side: PaneResizeSide, event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  activePaneResizeSide.value = side;
  paneResizeStartX = event.clientX;
  paneResizeStartWidth = side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value;
  paneResizePreviousBodyCursor = document.body.style.cursor;
  paneResizePreviousBodyUserSelect = document.body.style.userSelect;
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
  window.addEventListener("pointermove", handlePaneResizeMove);
  window.addEventListener("pointerup", stopPaneResize, { once: true });
  window.addEventListener("pointercancel", stopPaneResize, { once: true });
}

function handlePaneResizeMove(event: PointerEvent) {
  const side = activePaneResizeSide.value;
  if (!side) return;
  const pointerDelta = event.clientX - paneResizeStartX;
  const nextWidth = side === "left" ? paneResizeStartWidth + pointerDelta : paneResizeStartWidth - pointerDelta;
  setPaneWidth(side, nextWidth);
}

function stopPaneResize() {
  const side = activePaneResizeSide.value;
  window.removeEventListener("pointermove", handlePaneResizeMove);
  window.removeEventListener("pointerup", stopPaneResize);
  window.removeEventListener("pointercancel", stopPaneResize);
  document.body.style.cursor = paneResizePreviousBodyCursor;
  document.body.style.userSelect = paneResizePreviousBodyUserSelect;
  activePaneResizeSide.value = null;
  if (side) {
    storePaneWidth(side, side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value);
    emit("sidePanelWidthsCommit", {
      leftWidth: leftSidebarWidth.value,
      rightWidth: rightSidebarWidth.value,
    });
  }
}

function adjustPaneWidthByKeyboard(side: PaneResizeSide, delta: number) {
  const currentWidth = side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value;
  setPaneWidth(side, currentWidth + delta, true);
  emit("sidePanelWidthsCommit", {
    leftWidth: leftSidebarWidth.value,
    rightWidth: rightSidebarWidth.value,
  });
}

watch(
  [leftSidebarWidth, rightSidebarWidth],
  ([leftWidth, rightWidth]) => {
    emit("sidePanelWidthsChange", { leftWidth, rightWidth });
  },
  { immediate: true },
);

function handleDetachConversationRequest() {
  console.info("[独立聊天窗口][前端链路] ChatView 收到 detachConversation，继续派发到窗口容器", {
    activeConversationId: props.activeConversationId,
    detachedChatWindow: !!props.detachedChatWindow,
    isMainConversation: !!activeConversationSummary.value?.isMainConversation,
    conversationBusy: props.conversationBusy,
    frozen: props.frozen,
    chatting: props.chatting,
  });
  emit("detachConversation");
}

function handleSendChat() {
  const extraTextBlocks = attachedIdeContextReferences.value.map((item) => String(item.textBlock || "").trim()).filter(Boolean);
  emit("sendChat", extraTextBlocks.length > 0 ? { extraTextBlocks } : undefined);
  clearAttachedIdeContextReferences();
}

function openBranchSelectionMenu() {
  if (props.chatting || props.frozen || props.conversationBusy) return;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => {
    composerPanelRef.value?.focusInput?.({ preventScroll: true });
  });
}

function openDelegateSelectionMenu() {
  if (props.chatting || props.frozen || props.conversationBusy) return;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => {
    composerPanelRef.value?.openSelectionDelegateCard?.();
    composerPanelRef.value?.focusInput?.({ preventScroll: true });
  });
}

function openForwardSelectionMenu() {
  if (props.chatting || props.frozen || props.conversationBusy) return;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => {
    composerPanelRef.value?.openSelectionDeliverCard?.();
    composerPanelRef.value?.focusInput?.({ preventScroll: true });
  });
}

function openShareSelectionMenu() {
  if (props.chatting || props.frozen || props.conversationBusy) return;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => {
    composerPanelRef.value?.openSelectionShareCard?.();
    composerPanelRef.value?.focusInput?.({ preventScroll: true });
  });
}

const linkOpenErrorText = ref("");
const conversationSummaryCard = ref<{ visible: boolean; text: string }>({
  visible: false,
  text: "",
});
const composerPanelRef = ref<{
  focusInput: (options?: FocusOptions) => void;
  openSelectionDeliverCard?: () => void;
  openSelectionDelegateCard?: () => void;
  openSelectionShareCard?: () => void;
} | null>(null);
const messageSelectionModeEnabled = ref(false);
const selectedMessageRenderIds = ref<string[]>([]);
const olderHistoryRequestPending = ref(false);
const LOAD_OLDER_HISTORY_THRESHOLD_PX = 96;
const observedVirtualItemElements = new Map<string, HTMLElement>();
const observedVirtualItemResizeElements = new Map<string, HTMLElement>();
const measuredVirtualItemHeights = new Map<string, number>();
const streamingVirtualItemViewportTop = new Map<string, number>();
let pendingMeasureFrame = 0;
let pendingPinToBottomFrame = 0;
let activeJumpToBottomRequest = 0;
let pendingJumpToBottomFrame = 0;
let pendingProgrammaticScrollPaginationResetFrame = 0;
let lastConversationScrollTop = 0;
let virtualItemResizeObserver: ResizeObserver | null = null;
const olderHistoryTriggerReady = ref(true);
const suppressOlderHistoryPaginationOnce = ref(false);
const pendingOlderHistoryAnchor = ref<{ messageId: string; edge: "top" | "bottom"; offset: number } | null>(null);
const pendingOlderHistoryScrollRestore = ref<{ scrollTop: number; scrollHeight: number } | null>(null);
const timeDividerNowTick = ref(Date.now());
let timeDividerNowTimer = 0;

const {
  scrollContainer,
  composerContainer,
  toolbarContainer,
  chatLayoutRoot,
  latestOwnElasticMinHeight,
  showJumpToBottom,
  jumpToBottomStyle,
  onScroll,
} = useChatScrollLayout({
  activeConversationId: toRef(props, "activeConversationId"),
  chatting: toRef(props, "chatting"),
  busy: toRef(props, "conversationBusy"),
  frozen: toRef(props, "frozen"),
  messageBlockCount: computed(() => props.messageBlocks.length),
  onReachedBottom: () => emit("reachedBottom"),
  focusComposerInput: (options) => composerPanelRef.value?.focusInput(options),
});
const showSideConversationList = computed(() => !!props.sideConversationListVisible);

function refreshObservedVirtualItemElements() {
  const validIds = new Set(virtualRenderItems.value.map((item) => item.id));
  for (const [itemId] of observedVirtualItemElements.entries()) {
    if (!validIds.has(itemId)) {
      const resizeElement = observedVirtualItemResizeElements.get(itemId);
      if (resizeElement && virtualItemResizeObserver) {
        virtualItemResizeObserver.unobserve(resizeElement);
      }
      observedVirtualItemResizeElements.delete(itemId);
      observedVirtualItemElements.delete(itemId);
      measuredVirtualItemHeights.delete(itemId);
      streamingVirtualItemViewportTop.delete(itemId);
    }
  }
}

const {
  playingAudioId,
  copyMessage,
  stopAudioPlayback,
  toggleAudioPlayback,
} = useChatMessageActions();
const {
  isHidden: isBubbleBackgroundHidden,
  canToggle: canToggleBubbleBackground,
  toggle: toggleBubbleBackground,
} = useBubbleBackground(toRef(props, "activeConversationId"));

const selectedMessageRenderIdSet = computed(() => new Set(selectedMessageRenderIds.value));

const { chatRenderItems, messageMemoKey } = useChatVirtualList({
  messageBlocks: toRef(props, "messageBlocks"),
  markdownIsDark,
  playingAudioId,
  userAlias: toRef(props, "userAlias"),
  userAvatarUrl: toRef(props, "userAvatarUrl"),
  personaNameMap: toRef(props, "personaNameMap"),
  personaAvatarUrlMap: toRef(props, "personaAvatarUrlMap"),
  chatting: toRef(props, "chatting"),
  conversationBusy: toRef(props, "conversationBusy"),
  frozen: toRef(props, "frozen"),
  messageSelectionModeEnabled,
  selectedMessageRenderIdSet,
  isBubbleBackgroundHidden,
  canToggleBubbleBackground,
  canRegenerateBlock,
  canConfirmPlan,
});

const virtualRenderItems = computed<ChatRenderItem[]>(() => [...chatRenderItems.value]);

const virtualizer = useVirtualizer(
  computed(() => ({
    count: virtualRenderItems.value.length,
    getScrollElement: () => scrollContainer.value,
    getItemKey: (index: number) => virtualRenderItems.value[index]?.id ?? `row-${index}`,
    estimateSize: (index: number) => estimateChatRenderItemHeight(virtualRenderItems.value[index]),
    measureElement: (element: Element, _entry: unknown, instance: any) => {
      const measuredHeight = (element as HTMLElement).scrollHeight;
      if (instance?.scrollDirection !== "backward") return measuredHeight;
      const indexAttr = Number((element as HTMLElement).getAttribute("data-index"));
      const cachedHeight = Number.isFinite(indexAttr) ? instance?.itemSizeCache?.get(indexAttr) : undefined;
      return typeof cachedHeight === "number" ? cachedHeight : measuredHeight;
    },
    overscan: 4,
  })),
);

function blockBelongsToMessageId(block: ChatMessageBlock, messageId: string): boolean {
  const normalizedMessageId = String(messageId || "").trim();
  if (!normalizedMessageId) return false;
  const sourceMessageId = String(block.sourceMessageId || "").trim();
  const blockId = String(block.id || "").trim();
  return sourceMessageId === normalizedMessageId || blockId === normalizedMessageId;
}

function isDraftUserMessageId(messageId: string): boolean {
  return String(messageId || "").trim().startsWith("__draft_user__:");
}

const latestOwnMessageId = computed(() => {
  for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
    const block = props.messageBlocks[idx];
    if (block.isExtraTextBlock) continue;
    if (!isOwnMessage(block)) continue;
    const messageId = String(block.sourceMessageId || block.id || "").trim();
    if (messageId) return messageId;
  }
  return "";
});

const latestOwnElasticItemId = computed(() => {
  const targetMessageId = latestOwnMessageId.value;
  if (!targetMessageId) return "";
  for (let idx = chatRenderItems.value.length - 1; idx >= 0; idx -= 1) {
    const item = chatRenderItems.value[idx];
    if (item.kind === "message") {
      if (blockBelongsToMessageId(item.block, targetMessageId)) return item.id;
      continue;
    }
    if (item.kind === "group") {
      if (item.items.some((groupItem) => blockBelongsToMessageId(groupItem.block, targetMessageId))) {
        return item.id;
      }
    }
  }
  return "";
});

const renderItemChronologicalIndexMap = computed(() => {
  const map = new Map<string, number>();
  chatRenderItems.value.forEach((item, index) => {
    map.set(item.id, index);
  });
  return map;
});
const renderItemById = computed(() => {
  const map = new Map<string, ChatRenderItem>();
  chatRenderItems.value.forEach((item) => {
    map.set(item.id, item);
  });
  return map;
});
const blockChronologicalIndexMap = computed(() => {
  const map = new Map<string, number>();
  props.messageBlocks.forEach((block, index) => {
    const blockId = String(block.id || "").trim();
    if (!blockId || map.has(blockId)) return;
    map.set(blockId, index);
  });
  return map;
});

const virtualRows = computed(() => virtualizer.value.getVirtualItems());
const virtualEntries = computed(() => {
  return virtualRows.value
    .map((row) => {
      const item = virtualRenderItems.value[row.index];
      return item ? { row, item } : null;
    })
    .filter((entry): entry is { row: typeof virtualRows.value[number]; item: ChatRenderItem } => Boolean(entry));
});
const totalVirtualSize = computed(() => virtualizer.value.getTotalSize());

function isStreamingAssistantBlock(block: ChatMessageBlock): boolean {
  return !!block.isStreaming && !isOwnMessage(block);
}

function renderItemContainsStreamingAssistant(item: ChatRenderItem | undefined): boolean {
  if (!item) return false;
  if (item.kind === "message") return isStreamingAssistantBlock(item.block);
  if (item.kind === "group") return item.items.some((groupItem) => isStreamingAssistantBlock(groupItem.block));
  return false;
}

function elementTopInScrollViewport(scrollEl: HTMLElement, element: HTMLElement): number {
  const containerRect = scrollEl.getBoundingClientRect();
  const rect = element.getBoundingClientRect();
  return rect.top - containerRect.top;
}

function elementVisibleInScrollViewport(scrollEl: HTMLElement, element: HTMLElement): boolean {
  const containerRect = scrollEl.getBoundingClientRect();
  const rect = element.getBoundingClientRect();
  return rect.bottom > containerRect.top + 1 && rect.top < containerRect.bottom - 1;
}

function isNearBottomForStability(scrollEl: HTMLElement): boolean {
  const threshold = 24;
  return scrollEl.scrollHeight - (scrollEl.scrollTop + scrollEl.clientHeight) <= threshold;
}

function updateStreamingVirtualItemViewportTop(itemId: string, element?: HTMLElement | null) {
  const normalizedItemId = String(itemId || "").trim();
  if (!normalizedItemId) return;
  const scrollEl = scrollContainer.value;
  const target = element || observedVirtualItemElements.get(normalizedItemId) || null;
  const item = renderItemById.value.get(normalizedItemId);
  if (!scrollEl || !target || !target.isConnected || !renderItemContainsStreamingAssistant(item) || !elementVisibleInScrollViewport(scrollEl, target)) {
    streamingVirtualItemViewportTop.delete(normalizedItemId);
    return;
  }
  streamingVirtualItemViewportTop.set(normalizedItemId, elementTopInScrollViewport(scrollEl, target));
}

function syncVisibleStreamingVirtualItemViewportTops() {
  for (const [itemId, element] of observedVirtualItemElements.entries()) {
    updateStreamingVirtualItemViewportTop(itemId, element);
  }
}

function armProgrammaticScrollPaginationSuppression() {
  suppressOlderHistoryPaginationOnce.value = true;
  if (pendingProgrammaticScrollPaginationResetFrame) {
    cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
    pendingProgrammaticScrollPaginationResetFrame = 0;
  }
  pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
    pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
      suppressOlderHistoryPaginationOnce.value = false;
      pendingProgrammaticScrollPaginationResetFrame = 0;
    });
  });
}

function handleVirtualItemResize(element: HTMLElement) {
  const itemId = String(element.getAttribute("data-render-item-id") || "").trim();
  if (!itemId) return;
  const scrollEl = scrollContainer.value;
  const previousTop = streamingVirtualItemViewportTop.get(itemId);
  virtualizer.value.measureElement(element);
  const nextHeight = Math.round(element.getBoundingClientRect().height);
  measuredVirtualItemHeights.set(itemId, nextHeight);
  observedVirtualItemElements.set(itemId, element);
  if (
    scrollEl
    && previousTop !== undefined
    && !isNearBottomForStability(scrollEl)
    && renderItemContainsStreamingAssistant(renderItemById.value.get(itemId))
    && elementVisibleInScrollViewport(scrollEl, element)
  ) {
    const nextTop = elementTopInScrollViewport(scrollEl, element);
    const delta = nextTop - previousTop;
    if (Math.abs(delta) >= 1) {
      armProgrammaticScrollPaginationSuppression();
      scrollEl.scrollTop += delta;
      onScroll();
    }
  }
  updateStreamingVirtualItemViewportTop(itemId, element);
}

function scheduleVirtualMeasure() {
  if (pendingMeasureFrame) return;
  void nextTick(() => {
    if (pendingMeasureFrame) return;
    pendingMeasureFrame = requestAnimationFrame(() => {
      pendingMeasureFrame = 0;
      refreshObservedVirtualItemElements();
      syncVisibleStreamingVirtualItemViewportTops();
      virtualizer.value.measure();
      syncVisibleStreamingVirtualItemViewportTops();
      if (activeJumpToBottomRequest) scrollConversationToBottomOnce();
    });
  });
}

function measureVirtualRow(itemId: string, element: Element | ComponentPublicInstance | null) {
  const normalizedItemId = String(itemId || "").trim();
  if (!element) {
    if (normalizedItemId) {
      const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
      if (previousResizeElement && virtualItemResizeObserver) {
        virtualItemResizeObserver.unobserve(previousResizeElement);
      }
      observedVirtualItemResizeElements.delete(normalizedItemId);
      observedVirtualItemElements.delete(normalizedItemId);
      measuredVirtualItemHeights.delete(normalizedItemId);
      streamingVirtualItemViewportTop.delete(normalizedItemId);
    }
    return;
  }
  const target = element instanceof Element ? element : ((element.$el as Element | undefined) ?? null);
  if (!target) {
    if (normalizedItemId) {
      const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
      if (previousResizeElement && virtualItemResizeObserver) {
        virtualItemResizeObserver.unobserve(previousResizeElement);
      }
      observedVirtualItemResizeElements.delete(normalizedItemId);
      observedVirtualItemElements.delete(normalizedItemId);
      measuredVirtualItemHeights.delete(normalizedItemId);
      streamingVirtualItemViewportTop.delete(normalizedItemId);
    }
    return;
  }
  virtualizer.value.measureElement(target);
  const resolvedItemId = normalizedItemId || String(target.getAttribute("data-render-item-id") || "").trim();
  if (resolvedItemId && target instanceof HTMLElement) {
    const previousResizeElement = observedVirtualItemResizeElements.get(resolvedItemId);
    if (previousResizeElement && previousResizeElement !== target && virtualItemResizeObserver) {
      virtualItemResizeObserver.unobserve(previousResizeElement);
    }
    if (virtualItemResizeObserver && previousResizeElement !== target) {
      virtualItemResizeObserver.observe(target);
    }
    observedVirtualItemResizeElements.set(resolvedItemId, target);
    const nextHeight = Math.round(target.getBoundingClientRect().height);
    measuredVirtualItemHeights.set(resolvedItemId, nextHeight);
    observedVirtualItemElements.set(resolvedItemId, target);
    updateStreamingVirtualItemViewportTop(resolvedItemId, target);
  }
}

function scrollConversationToBottomOnce() {
  const scrollEl = scrollContainer.value;
  if (!scrollEl) return;
  scrollEl.scrollTop = scrollEl.scrollHeight;
  onScroll();
  chatScrollbarRef.value?.updateThumb();
}

function scheduleJumpToBottomStep(requestId: number, remainingFrames: number) {
  if (activeJumpToBottomRequest !== requestId) return;
  if (pendingJumpToBottomFrame) {
    cancelAnimationFrame(pendingJumpToBottomFrame);
    pendingJumpToBottomFrame = 0;
  }
  pendingJumpToBottomFrame = requestAnimationFrame(() => {
    pendingJumpToBottomFrame = 0;
    if (activeJumpToBottomRequest !== requestId) return;
    scheduleVirtualMeasure();
    scrollConversationToBottomOnce();
    if (remainingFrames > 0) {
      scheduleJumpToBottomStep(requestId, remainingFrames - 1);
      return;
    }
    activeJumpToBottomRequest = 0;
  });
}

function startJumpToBottomTransaction() {
  activeJumpToBottomRequest += 1;
  armProgrammaticScrollPaginationSuppression();
  pendingOlderHistoryAnchor.value = null;
  pendingOlderHistoryScrollRestore.value = null;
  scheduleVirtualMeasure();
  void nextTick(() => {
    scrollConversationToBottomOnce();
    requestAnimationFrame(() => {
      scrollConversationToBottomOnce();
      activeJumpToBottomRequest = 0;
    });
  });
}
function pinToBottomOnNextLayout(smooth = false, reason = "unknown") {
  if (props.chatting && reason !== "activeConversationChanged") return;
  if (pendingPinToBottomFrame) {
    cancelAnimationFrame(pendingPinToBottomFrame);
    pendingPinToBottomFrame = 0;
  }
  void nextTick(() => {
    scheduleVirtualMeasure();
    pendingPinToBottomFrame = requestAnimationFrame(() => {
      pendingPinToBottomFrame = 0;
      const scrollEl = scrollContainer.value;
      if (!scrollEl) return;
      requestAnimationFrame(() => {
        if (activeJumpToBottomRequest) return;
        scrollEl.scrollTo({
          top: scrollEl.scrollHeight,
          behavior: smooth ? "smooth" : "auto",
        });
        onScroll();
      });
    });
  });
}

function alignLatestOwnMessageToTop(behavior: ScrollBehavior = "smooth") {
  const scrollEl = scrollContainer.value;
  const itemId = String(latestOwnElasticItemId.value || "").trim();
  if (!scrollEl || !itemId) return;
  const wrapper = observedVirtualItemElements.get(itemId);
  if (!wrapper || !wrapper.isConnected) return;
  const containerRect = scrollEl.getBoundingClientRect();
  const wrapperRect = wrapper.getBoundingClientRect();
  const scrollStyles = window.getComputedStyle(scrollEl);
  const targetTop = parseFloat(scrollStyles.paddingTop || "0");
  const nextTop = scrollEl.scrollTop + (wrapperRect.top - containerRect.top) - targetTop;
  scrollEl.scrollTo({
    top: Math.max(0, nextTop),
    behavior,
  });
  onScroll();
}

function syncViewportMetrics() {
  scheduleVirtualMeasure();
  void nextTick(() => chatScrollbarRef.value?.updateThumb());
}

function findRenderedMessageElement(messageId: string): HTMLElement | null {
  const scrollEl = scrollContainer.value;
  const normalizedId = String(messageId || "").trim();
  if (!scrollEl || !normalizedId) return null;
  const escapedId = typeof CSS !== "undefined" && typeof CSS.escape === "function"
    ? CSS.escape(normalizedId)
    : normalizedId.replace(/["\\]/g, "\\$&");
  return scrollEl.querySelector(`[data-message-id="${escapedId}"]`) as HTMLElement | null;
}

function resolveMessageAnchorElement(messageElement: HTMLElement | null): HTMLElement | null {
  if (!messageElement) return null;
  return (messageElement.querySelector("[data-message-avatar-anchor='true']") as HTMLElement | null) || messageElement;
}

function captureVisibleAnchor(edge: "top" | "bottom"): { messageId: string; edge: "top" | "bottom"; offset: number } | null {
  const scrollEl = scrollContainer.value;
  if (!scrollEl) return null;
  const containerRect = scrollEl.getBoundingClientRect();
  let anchor: { messageId: string; offset: number; chronologicalIndex: number } | null = null;
  for (const entry of virtualEntries.value) {
    const itemId = String(entry.item.id || "").trim();
    if (!itemId) continue;
    const wrapper = observedVirtualItemElements.get(itemId);
    if (!wrapper || !wrapper.isConnected) continue;
    const messageElements = Array.from(wrapper.querySelectorAll("[data-message-id]")) as HTMLElement[];
    for (const element of messageElements) {
      const messageId = String(element.getAttribute("data-message-id") || "").trim();
      if (!messageId) continue;
      const anchorElement = resolveMessageAnchorElement(element);
      if (!anchorElement) continue;
      const rect = anchorElement.getBoundingClientRect();
      if (rect.bottom <= containerRect.top + 1 || rect.top >= containerRect.bottom - 1) continue;
      const chronologicalIndex = blockChronologicalIndexMap.value.get(messageId);
      if (chronologicalIndex === undefined) continue;
      const offset = edge === "bottom"
        ? containerRect.bottom - rect.bottom
        : rect.top - containerRect.top;
      if (!anchor) {
        anchor = { messageId, offset, chronologicalIndex };
        continue;
      }
      const shouldReplace = edge === "bottom"
        ? chronologicalIndex > anchor.chronologicalIndex
        : chronologicalIndex < anchor.chronologicalIndex;
      if (shouldReplace) {
        anchor = { messageId, offset, chronologicalIndex };
      }
    }
  }
  return anchor ? { messageId: anchor.messageId, edge, offset: anchor.offset } : null;
}

function onConversationScroll() {
  const scrollEl = scrollContainer.value;
  onScroll();
  chatScrollbarRef.value?.updateThumb();
  if (suppressOlderHistoryPaginationOnce.value) {
    suppressOlderHistoryPaginationOnce.value = false;
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
  } else {
    maybeRequestOlderHistory();
  }
  if (scrollEl) {
    lastConversationScrollTop = scrollEl.scrollTop;
  }
  syncVisibleStreamingVirtualItemViewportTops();
}

function handleJumpToBottom() {
  startJumpToBottomTransaction();
  if (props.chatting || props.conversationBusy || props.frozen) return;
  emit("jumpToConversationBottom");
}

function maybeRequestOlderHistory() {
  const scrollEl = scrollContainer.value;
  if (!scrollEl) return;
  if (!props.hasMoreHistory || props.loadingOlderHistory || olderHistoryRequestPending.value) return;
  if (!olderHistoryTriggerReady.value) return;
  if (scrollEl.scrollTop > LOAD_OLDER_HISTORY_THRESHOLD_PX) return;
  const isMovingUpward = scrollEl.scrollTop <= lastConversationScrollTop;
  if (!isMovingUpward) return;
  pendingOlderHistoryScrollRestore.value = {
    scrollTop: scrollEl.scrollTop,
    scrollHeight: scrollEl.scrollHeight,
  };
  pendingOlderHistoryAnchor.value = captureVisibleAnchor("bottom");
  olderHistoryRequestPending.value = true;
  olderHistoryTriggerReady.value = false;
  emit("loadOlderHistory");
}

watch(showSideConversationList, () => syncViewportMetrics(), { immediate: true });

function handleConversationPinToggle(conversationId: string) {
  emit("togglePinConversation", String(conversationId || "").trim());
}

function handleConversationArchive(conversationId: string) {
  emit("archiveConversation", String(conversationId || "").trim());
}

function handleConversationDelete(conversationId: string) {
  emit("deleteConversation", String(conversationId || "").trim());
}
watch(
  () => String(props.activeConversationId || "").trim(),
  () => {
    exitMessageSelectionMode();
    chatScrollbarRef.value?.hide();
    pinToBottomOnNextLayout(false, "activeConversationChanged");
    olderHistoryRequestPending.value = false;
    lastConversationScrollTop = 0;
    olderHistoryTriggerReady.value = true;
    pendingOlderHistoryAnchor.value = null;
    pendingOlderHistoryScrollRestore.value = null;
  },
  { immediate: true },
);

watch(
  () => props.conversationScrollToBottomRequest,
  (nextValue, prevValue) => {
    if (!nextValue || nextValue === prevValue) return;
    startJumpToBottomTransaction();
  },
);

watch(
  () => props.latestOwnMessageAlignRequest,
  (nextValue, prevValue) => {
    if (!nextValue || nextValue === prevValue) return;
    if (isDraftUserMessageId(latestOwnMessageId.value)) return;
    void nextTick(async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      refreshObservedVirtualItemElements();
      alignLatestOwnMessageToTop("smooth");
    });
  },
);

watch(
  latestOwnMessageId,
  (nextValue, prevValue) => {
    if (!prevValue || !nextValue) return;
    if (!isDraftUserMessageId(prevValue) || isDraftUserMessageId(nextValue)) return;
    void nextTick(async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      refreshObservedVirtualItemElements();
      alignLatestOwnMessageToTop("smooth");
    });
  },
);

watch(
  () => props.messageBlocks.length,
  () => {
    refreshObservedVirtualItemElements();
    void nextTick(() => chatScrollbarRef.value?.updateThumb());
  },
);

watch(
  () => props.loadingOlderHistory,
  async (loading, wasLoading) => {
    if (loading) return;
    if (!wasLoading) return;
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    refreshObservedVirtualItemElements();
    const scrollRestore = pendingOlderHistoryScrollRestore.value;
    if (scrollRestore) {
      const deltaHeight = scrollEl.scrollHeight - scrollRestore.scrollHeight;
      scrollEl.scrollTop = Math.max(0, scrollRestore.scrollTop + deltaHeight);
    }
    const anchor = pendingOlderHistoryAnchor.value;
    if (anchor) {
      const anchorMessageElement = findRenderedMessageElement(anchor.messageId);
      const anchorElement = resolveMessageAnchorElement(anchorMessageElement);
      if (anchorElement && anchorElement.isConnected) {
        const containerRect = scrollEl.getBoundingClientRect();
        const rect = anchorElement.getBoundingClientRect();
        if (anchor.edge === "bottom") {
          const currentOffset = containerRect.bottom - rect.bottom;
          scrollEl.scrollTop += anchor.offset - currentOffset;
        } else {
          const currentOffset = rect.top - containerRect.top;
          scrollEl.scrollTop += currentOffset - anchor.offset;
        }
      }
    }
    lastConversationScrollTop = scrollEl.scrollTop;
    olderHistoryRequestPending.value = false;
    olderHistoryTriggerReady.value = true;
    pendingOlderHistoryAnchor.value = null;
    pendingOlderHistoryScrollRestore.value = null;
  },
);
const {
  imagePreviewOpen,
  imagePreviewDataUrl,
  imagePreviewZoom,
  IMAGE_PREVIEW_MIN_ZOOM,
  IMAGE_PREVIEW_MAX_ZOOM,
  previewOffsetX,
  previewOffsetY,
  previewDragging,
  zoomInPreview,
  zoomOutPreview,
  resetPreviewZoom,
  onPreviewWheel,
  openImagePreview,
  closeImagePreview,
  onPreviewPointerDown,
  onPreviewPointerMove,
  onPreviewPointerUp,
} = useChatImagePreview();
const renderedMessageItems = computed(() =>
  chatRenderItems.value.flatMap((item) => {
    if (item.kind === "message") {
      return [{ renderId: item.renderId, block: item.block }];
    }
    if (item.kind === "group") {
      return item.items.map((groupItem) => ({ renderId: groupItem.renderId, block: groupItem.block }));
    }
    return [];
  }),
);
const selectedMessageBlocks = computed(() =>
  renderedMessageItems.value.filter((item) => selectedMessageRenderIdSet.value.has(item.renderId)),
);
watch(
  () => ({
    selectionModeEnabled: messageSelectionModeEnabled.value,
    selectedRenderIdCount: selectedMessageRenderIds.value.length,
    selectedBlockCount: selectedMessageBlocks.value.length,
  }),
  ({ selectionModeEnabled, selectedRenderIdCount, selectedBlockCount }) => {
    if (!selectionModeEnabled) return;
    if (selectedRenderIdCount === 0) return;
    if (selectedBlockCount === 0) {
      exitMessageSelectionMode();
    }
  },
);
const {
  toolReviewPanelOpen,
  toolReviewBatches,
  toolReviewCurrentBatchKey,
  toolReviewButtonCount,
  toolReviewButtonLabel,
  toolReviewButtonEnabled,
  toolReviewDetailMap,
  toolReviewDetailLoadingCallId,
  toolReviewReviewingCallId,
  toolReviewBatchReviewingKey,
  toolReviewSubmittingBatchKey,
  toolReviewErrorText,
  toolReviewReportErrorText,
  toolReviewReports,
  toolReviewCurrentReportId,
  toggleToolReviewPanel,
  refreshToolReviewBatches,
  refreshToolReviewReports,
  setToolReviewCurrentBatchKey,
  loadToolReviewItemDetail,
  runToolReviewForCall,
  runToolReviewForBatch,
  submitToolReviewCode,
  deleteToolReviewReport,
  listToolReviewCommitOptions,
} = useChatToolReview({
  activeConversationId: toRef(props, "activeConversationId"),
  messageBlocks: computed(() => props.messageBlocks),
  refreshTick: toRef(props, "toolReviewRefreshTick"),
  t,
  onRefreshMessage: (payload) => emit("refreshToolReviewMessage", payload),
});
watch(
  toolReviewPanelOpen,
  (value) => {
    emit("toolReviewPanelOpenChange", value);
    syncViewportMetrics();
  },
  { immediate: true },
);
const {
  delegateStatuses,
  delegateStatusesLoading,
  delegateStatusesErrorText,
  openDelegateArchiveDetail,
} = useDelegateStatus({
  activeConversationId: toRef(props, "activeConversationId"),
  panelOpen: toolReviewPanelOpen,
});

function isOwnMessage(block: ChatMessageBlock): boolean {
  return isRightAlignedMessage(block);
}

function canRegenerateBlock(block: ChatMessageBlock, blockIndex: number): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
    const candidate = props.messageBlocks[idx];
    if (candidate.role !== "assistant" || candidate.isExtraTextBlock) continue;
    return idx === blockIndex;
  }
  return false;
}

function canConfirmPlan(block: ChatMessageBlock): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  if (block.planCard?.action !== "present") return false;
  const targetId = String(block.sourceMessageId || block.id || "").trim();
  if (targetId !== latestPendingPlanMessageId.value) return false;
  const blockIndex = props.messageBlocks.findIndex((item) => String(item.id || "").trim() === String(block.id || "").trim());
  if (blockIndex < 0) return false;
  return !props.messageBlocks.slice(blockIndex + 1).some((item) => !item.isExtraTextBlock && item.role === "user");
}

function handleConversationListSelect(payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }) {
  const normalizedConversationId = String(payload?.conversationId || "").trim();
  if (!normalizedConversationId) return;
  const isCurrent = normalizedConversationId === String(props.activeConversationId || "").trim();
  if (isCurrent) return;
  const target = (props.conversationItems || props.unarchivedConversationItems).find(
    (item) => String(item.conversationId || "").trim() === normalizedConversationId,
  );
  emit("switchConversation", {
    conversationId: normalizedConversationId,
    kind: payload?.kind || target?.kind,
    remoteContactId: String(payload?.remoteContactId || target?.remoteContactId || "").trim() || undefined,
  });
}

function handleConversationRename(payload: { conversationId: string; title: string }) {
  const conversationId = String(payload?.conversationId || "").trim();
  const title = String(payload?.title || "").trim();
  if (!conversationId) return;
  emit("renameConversation", {
    conversationId,
    title,
  });
}

function openConversationSummary(block: ChatMessageBlock, event?: MouseEvent) {
  event?.stopPropagation();
  const text = String(block?.text || "").trim();
  if (!text) return;
  conversationSummaryCard.value = {
    visible: true,
    text,
  };
}

function closeConversationSummaryCard() {
  conversationSummaryCard.value = {
    visible: false,
    text: "",
  };
}

function enterMessageSelectionMode(selectionKey: string) {
  const normalizedSelectionKey = String(selectionKey || "").trim();
  if (!normalizedSelectionKey) return;
  messageSelectionModeEnabled.value = true;
  if (!selectedMessageRenderIds.value.includes(normalizedSelectionKey)) {
    selectedMessageRenderIds.value = [...selectedMessageRenderIds.value, normalizedSelectionKey];
  }
}

function toggleMessageSelected(selectionKey: string) {
  const normalizedSelectionKey = String(selectionKey || "").trim();
  if (!normalizedSelectionKey) return;
  if (!messageSelectionModeEnabled.value) {
    enterMessageSelectionMode(normalizedSelectionKey);
    return;
  }
  if (selectedMessageRenderIds.value.includes(normalizedSelectionKey)) {
    selectedMessageRenderIds.value = selectedMessageRenderIds.value.filter((item) => item !== normalizedSelectionKey);
    return;
  }
  selectedMessageRenderIds.value = [...selectedMessageRenderIds.value, normalizedSelectionKey];
}

function exitMessageSelectionMode() {
  messageSelectionModeEnabled.value = false;
  selectedMessageRenderIds.value = [];
}

defineExpose({
  exitMessageSelectionMode,
});

function selectionPayload() {
  const blocks = selectedMessageBlocks.value.map((item) => item.block);
  return {
    count: blocks.length,
    messageIds: blocks
      .map((block) => String(block.sourceMessageId || block.id || "").trim())
      .filter((item) => !!item),
    blocks,
  };
}

function selectionDisplayName(block: ChatMessageBlock): string {
  if (block.remoteImOrigin) {
    return block.remoteImOrigin.senderName || block.remoteImOrigin.remoteContactName || "IM";
  }
  const speakerAgentId = String(block.speakerAgentId || "").trim();
  if (speakerAgentId && props.personaNameMap[speakerAgentId]) {
    return props.personaNameMap[speakerAgentId];
  }
  if (!speakerAgentId || speakerAgentId === "user-persona" || block.role === "user") {
    return props.userAlias || t("archives.roleUser");
  }
  return speakerAgentId || block.role;
}

function selectionBlockSummary(block: ChatMessageBlock): string {
  const parts: string[] = [];
  const text = String(block.text || "").trim();
  if (text) {
    parts.push(text);
  }
  if (block.images.length > 0) {
    parts.push(t("chat.imageCount", { count: block.images.length }));
  }
  if (block.audios.length > 0) {
    parts.push(t("chat.audioCount", { count: block.audios.length }));
  }
  if (block.attachmentFiles.length > 0) {
    parts.push(
      t("chat.attachmentList", {
        names: block.attachmentFiles.map((item) => item.fileName).join("、"),
      }),
    );
  }
  return parts.join("\n").trim();
}

async function copySelectedMessages() {
  const payload = selectionPayload();
  if (payload.count === 0) return;
  const text = payload.blocks
    .map((block) => {
      const body = selectionBlockSummary(block);
      return `[${selectionDisplayName(block)}]: ${body || "[空消息]"}`;
    })
    .join("\n\n");
  if (!text.trim()) return;
  try {
    await navigator.clipboard.writeText(text);
    emit("selectionActionCopy", payload);
  } catch (error) {
    emit("selectionActionCopyError", {
      ...payload,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

async function copyToolReviewReport(reportText: string) {
  const text = String(reportText || "").trim();
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // 剪贴板失败不阻断报告查看。
  }
}

function emitSelectionAction(
  kind: "branch" | "share" | "forward" | "delegate",
  actionPayload: string | { departmentId: string; presetId: string; background: string; question: string; focus: string } = "",
) {
  const payload = selectionPayload();
  if (kind === "branch") {
    if (payload.count === 0) return;
    emit("selectionActionBranch", payload);
    return;
  }
  if (kind === "forward") {
    if (payload.count === 0) return;
    const normalizedTargetConversationId = String(actionPayload || "").trim();
    if (!normalizedTargetConversationId) return;
    emit("selectionActionForward", {
      ...payload,
      targetConversationId: normalizedTargetConversationId,
    });
    return;
  }
  if (kind === "delegate") {
    if (!actionPayload || typeof actionPayload === "string") return;
    emit("selectionActionDelegate", {
      ...payload,
      departmentId: String(actionPayload.departmentId || "").trim(),
      presetId: String(actionPayload.presetId || "review").trim() || "review",
      background: String(actionPayload.background || "").trim(),
      question: String(actionPayload.question || "").trim(),
      focus: String(actionPayload.focus || "").trim(),
    });
    return;
  }
  if (payload.count === 0) return;
  const exportFormat = actionPayload === "html" || actionPayload === "png" ? actionPayload : undefined;
  emit("selectionActionShare", {
    ...payload,
    exportFormat,
  });
}

async function handleAssistantLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const anchor = target?.closest("a") as HTMLAnchorElement | null;
  if (!anchor) return;
  const href = normalizeLocalLinkHref(anchor.getAttribute("href")?.trim() || "");
  if (!href) return;

  if (isAbsoluteLocalPath(href)) {
    event.preventDefault();
    event.stopPropagation();
    try {
      if (canOpenInFileReader(href)) {
        await invokeTauri("open_file_reader_window_command", { path: href });
      } else {
        await invokeTauri("open_local_file_directory", { path: href });
      }
      linkOpenErrorText.value = "";
    } catch (error) {
      linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) });
    }
    return;
  }

  if (href.startsWith("http://") || href.startsWith("https://")) {
    event.preventDefault();
    event.stopPropagation();
    try {
      await invokeTauri("open_external_url", { url: href });
      linkOpenErrorText.value = "";
    } catch (error) {
      linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) });
    }
  }
}

function padTimePart(value: number): string {
  return String(value).padStart(2, "0");
}

function formatLocalClock(date: Date): string {
  return `${padTimePart(date.getHours())}:${padTimePart(date.getMinutes())}`;
}

function formatTimeDividerLabel(value?: string): string {
  const raw = String(value || "").trim();
  if (!raw) return "";
  const date = new Date(raw);
  const timestamp = date.getTime();
  if (!Number.isFinite(timestamp)) return raw;

  const now = new Date(timeDividerNowTick.value);
  const sameYear = date.getFullYear() === now.getFullYear();
  const sameMonth = date.getMonth() === now.getMonth();
  const sameDate = date.getDate() === now.getDate();
  const sameHour = sameYear && sameMonth && sameDate && date.getHours() === now.getHours();
  const elapsedMinutes = Math.floor((now.getTime() - timestamp) / 60000);
  if (sameHour && elapsedMinutes > 0) {
    return t("chat.timeDivider.minutesAgo", { count: elapsedMinutes });
  }

  const clock = formatLocalClock(date);
  if (sameYear && sameMonth && sameDate) return clock;
  const monthDay = `${padTimePart(date.getMonth() + 1)}-${padTimePart(date.getDate())}`;
  if (sameYear) return `${monthDay} ${clock}`;
  return `${date.getFullYear()}-${monthDay} ${clock}`;
}

onMounted(() => {
  timeDividerNowTimer = window.setInterval(() => {
    timeDividerNowTick.value = Date.now();
  }, 60_000);
  void nextTick(() => chatScrollbarRef.value?.updateThumb());
  if (typeof ResizeObserver !== "undefined") {
    virtualItemResizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (!(entry.target instanceof HTMLElement)) continue;
        handleVirtualItemResize(entry.target);
      }
    });
    for (const element of observedVirtualItemResizeElements.values()) {
      if (!element.isConnected) continue;
      virtualItemResizeObserver.observe(element);
    }
    syncVisibleStreamingVirtualItemViewportTops();
  }
});

onBeforeUnmount(() => {
  if (timeDividerNowTimer) {
    window.clearInterval(timeDividerNowTimer);
    timeDividerNowTimer = 0;
  }
  stopPaneResize();
  virtualItemResizeObserver?.disconnect();
  virtualItemResizeObserver = null;
  if (pendingMeasureFrame) {
    cancelAnimationFrame(pendingMeasureFrame);
    pendingMeasureFrame = 0;
  }
  if (pendingPinToBottomFrame) {
    cancelAnimationFrame(pendingPinToBottomFrame);
    pendingPinToBottomFrame = 0;
  }
  if (pendingJumpToBottomFrame) {
    cancelAnimationFrame(pendingJumpToBottomFrame);
    pendingJumpToBottomFrame = 0;
  }
  activeJumpToBottomRequest = 0;
  if (pendingProgrammaticScrollPaginationResetFrame) {
    cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
    pendingProgrammaticScrollPaginationResetFrame = 0;
  }
  observedVirtualItemElements.clear();
  observedVirtualItemResizeElements.clear();
  measuredVirtualItemHeights.clear();
  streamingVirtualItemViewportTop.clear();
  stopAudioPlayback();
});

</script>

<style scoped>
.scrollbar-gutter-stable {
  scrollbar-gutter: stable;
}

.conversation-tray-scroll-hidden {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.conversation-tray-scroll-hidden::-webkit-scrollbar {
  display: none;
}

.ecall-pane-splitter {
  position: relative;
  z-index: 20;
  width: 6px;
  flex: 0 0 6px;
  cursor: col-resize;
  touch-action: none;
  outline: none;
}

.ecall-pane-splitter::after {
  content: "";
  position: absolute;
  inset: 0 2px;
  border-radius: 999px;
  background: transparent;
  transition: background-color 120ms ease, inset 120ms ease;
}

.ecall-pane-splitter:hover::after,
.ecall-pane-splitter:focus-visible::after,
.ecall-pane-splitter-active::after {
  inset: 8px 1px;
  background: hsl(var(--bc) / 0.18);
}

.ecall-pane-splitter-left {
  margin-left: -3px;
  margin-right: -3px;
}

.ecall-pane-splitter-right {
  margin-left: -3px;
  margin-right: -3px;
}

.ecall-chat-scroll-container {
  overscroll-behavior-y: contain;
  overflow-anchor: none;
  scrollbar-gutter: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.ecall-chat-scroll-container::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.ecall-chat-scroll-container[data-chat-interaction-locked="true"] :deep(.ecall-message-footer),
.ecall-chat-scroll-container[data-chat-interaction-locked="true"] :deep(.ecall-message-footer-action),
.ecall-chat-scroll-container[data-chat-interaction-locked="true"] :deep(.ecall-message-recall-action) {
  opacity: 0 !important;
  pointer-events: none !important;
}

.ecall-chat-scroll-container[data-chat-interaction-locked="true"] :deep(.ecall-plan-confirm-action) {
  pointer-events: none !important;
}

.ecall-turn-group {
  display: block;
  width: 100%;
}

.ecall-chat-virtual-list {
  width: 100%;
  min-width: 0;
}

.ecall-chat-virtual-item {
  display: block;
  width: 100%;
  min-width: 0;
}

.ecall-chat-virtual-spacer {
  width: 100%;
  flex: 0 0 auto;
}

.ecall-turn-stack {
  display: flow-root;
  width: 100%;
  min-height: 0;
}

.ecall-elastic-item-shell {
  width: 100%;
  min-height: 0;
}


.ecall-chat-avatar-col {
  width: 1.75rem;
  min-width: 1.75rem;
}

.ecall-time-loading {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  transform: scale(0.82);
  transform-origin: right center;
}

:deep(.chat-bubble) {
  min-width: 0;
  min-height: 0;
}

.ecall-assistant-bubble {
  min-width: 3rem;
  min-height: 2.25rem;
}

.ecall-assistant-bubble-wide {
  width: 100%;
  max-width: 100%;
}

:deep(.chat-input-no-focus),
:deep(.chat-input-no-focus:hover),
:deep(.chat-input-no-focus:focus),
:deep(.chat-input-no-focus:focus-visible) {
  border: none !important;
  outline: none !important;
  box-shadow: none !important;
}

</style>
