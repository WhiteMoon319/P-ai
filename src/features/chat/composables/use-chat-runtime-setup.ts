import { onScopeDispose } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import { registerChatFlowRuntime } from "./chat-flow-runtime-registry";
import { useChatFlow } from "./use-chat-flow";
import { useChatRewindActions } from "./use-chat-rewind-actions";
import { useConfirmPlan } from "./use-confirm-plan";

export function useChatRuntimeSetup(bindings: Record<string, any>) {
  let chatFlowRef: any = null;

  const chatFlow = useChatFlow({
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      isConversationBusy: () => {
        const conversationId = String(bindings.currentChatConversationId.value || "").trim();
        if (!conversationId) return false;
        const runtimeState = String(
          typeof bindings.currentConversationRuntimeState === "function"
            ? bindings.currentConversationRuntimeState(conversationId)
            : "",
        ).trim();
        if (runtimeState === "assistant_streaming" || runtimeState === "organizing_context" || runtimeState === "compacting") {
          return true;
        }
        const trimmingId = String(bindings.trimmingConversationId?.value || "").trim();
        if (bindings.trimming?.value && (!trimmingId || trimmingId === conversationId)) {
          return true;
        }
        const compactingId = String(bindings.compactingConversationId?.value || "").trim();
        return !!bindings.compactingConversation?.value && (!compactingId || compactingId === conversationId);
      },
      getSession: () => {
        const apiConfigId = String(bindings.currentForegroundApiConfigId.value || "").trim();
        const agentId = String(bindings.currentForegroundAgentId.value || "").trim();
        const departmentId = String(bindings.currentForegroundDepartmentId.value || "").trim();
        if (!apiConfigId || !agentId) return null;
        return { apiConfigId, agentId, departmentId };
      },
      getConversationId: () => String(bindings.currentChatConversationId.value || "").trim(),
      chatInput: bindings.chatInput,
      selectedMentions: bindings.selectedChatMentions,
      clipboardImages: bindings.clipboardImages,
      queuedAttachmentNotices: bindings.queuedAttachmentNotices,
      latestUserText: bindings.latestUserText,
      latestUserImages: bindings.latestUserImages,
      latestAssistantText: bindings.latestAssistantText,
      toolStatusText: bindings.toolStatusText,
      toolStatusState: bindings.toolStatusState,
      streamBlocks: bindings.streamBlocks,
      contextUsagePreview: bindings.latestContextUsagePreview,
      chatErrorText: bindings.chatErrorText,
      setConversationChatError: bindings.setConversationChatErrorText,
      allMessages: bindings.allMessages,
      onOwnUserDraftInserted: ({ conversationId }) => {
        const insertedConversationId = String(conversationId || "").trim();
        if (
          insertedConversationId
          && bindings.isChatWindowActiveNow()
          && !String(bindings.currentChatConversationId.value || "").trim()
        ) {
          bindings.currentChatConversationId.value = insertedConversationId;
        }
        bindings.bumpOwnUserDraftAlign();
        bindings.cacheConversationMessages(
          insertedConversationId || String(bindings.currentChatConversationId.value || "").trim(),
          bindings.allMessages.value,
        );
      },
      onAssistantDraftInserted: () => {
        bindings.bumpOwnUserDraftAlign();
      },
      t: bindings.tr,
      formatRequestFailed: (error: unknown) => bindings.formatRequestFailed(error),
      removeBinaryPlaceholders: bindings.removeBinaryPlaceholders,
      invokeSendChatMessage: ({ text, displayText, parts, extraTextBlocks, mentions, session, traceId, onDelta }) =>
        invokeTauri(
          "submit_chat_message",
          {
            input: {
              payload: {
                text,
                displayText,
                parts,
                extraTextBlocks: extraTextBlocks && extraTextBlocks.length > 0 ? extraTextBlocks : undefined,
                mentions: Array.isArray(mentions) && mentions.length > 0
                  ? mentions.map((item: any) => ({
                      agentId: item.agentId,
                      agentName: item.agentName,
                      departmentId: item.departmentId,
                      departmentName: item.departmentName,
                    }))
                  : undefined,
              },
              session: {
                apiConfigId: session.apiConfigId,
                agentId: session.agentId,
                departmentId: session.departmentId || null,
                conversationId: session.conversationId || null,
              },
              traceId,
            },
            onDelta,
          },
        ),
      invokeStopChatMessage: ({ session, partialAssistantText, partialStreamBlocks }) =>
        invokeTauri("stop_chat_message", {
          input: {
            session: {
              apiConfigId: session.apiConfigId,
              agentId: session.agentId,
              departmentId: session.departmentId || null,
              conversationId: session.conversationId || null,
            },
            partialAssistantText,
            partialStreamBlocks,
          },
        }),
      refreshMessageById: async ({ conversationId, messageId }) => {
        const normalizedMessageId = String(messageId || "").trim();
        const beforeMessage = bindings.allMessages.value.find((message: any) => String(message?.id || "").trim() === normalizedMessageId);
        await bindings.refreshForegroundConversationMessageById({
          conversationId,
          messageId,
        });
        const afterMessage = bindings.allMessages.value.find((message: any) => String(message?.id || "").trim() === normalizedMessageId);
        return !!afterMessage && afterMessage !== beforeMessage;
      },
      invokeBindActiveChatViewStream: ({ bindingId, conversationId, onDelta }) =>
        invokeTauri("bind_active_chat_view_stream", {
          input: {
            bindingId,
            conversationId: conversationId || null,
          },
          onDelta,
        }),
      invokeUnbindActiveChatViewStream: ({ bindingId }) =>
        invokeTauri("unbind_active_chat_view_stream", {
          input: { bindingId },
        }),
      invokeProbeActiveChatViewStream: ({ bindingId, conversationId, probeId }) =>
        invokeTauri<boolean>("probe_active_chat_view_stream", {
          input: {
            bindingId,
            conversationId: conversationId || null,
            probeId,
          },
        }),
      onReloadMessages: () => bindings.reloadForegroundConversationMessages("chat_flow_reload"),
      onAssistantMessageCompleted: async ({ conversationId, assistantMessage }) => {
        bindings.applyConversationMessageAppended({
          conversationId,
          message: assistantMessage,
        });
      },
      onHistoryFlushed: async ({ conversationId, pendingMessages }) => {
        const flushedConversationId = String(conversationId || "").trim();
        if (flushedConversationId && bindings.isChatWindowActiveNow()) {
          bindings.currentChatConversationId.value = flushedConversationId;
        }
        const queueMessages = Array.isArray(pendingMessages) ? pendingMessages : [];
        if (queueMessages.length > 0) {
          const fastPathResult = bindings.applySingleOwnUserHistoryFlushFastPath(queueMessages);
          if (fastPathResult) {
            bindings.cacheConversationMessages(
              flushedConversationId || String(bindings.currentChatConversationId.value || "").trim(),
              bindings.allMessages.value,
            );
            return;
          }
          const currentMessages = [...bindings.allMessages.value];
          const dedup = new Set(
            currentMessages
              .filter((message: any) => !bindings.isOptimisticOwnUserDraft(message))
              .map((message: any) => String(message.id || "").trim())
              .filter((id: string) => !!id),
          );
          const uniqueIncoming = queueMessages.filter((message: any) => {
            const id = String(message.id || "").trim();
            if (!id) return true;
            if (dedup.has(id)) return false;
            dedup.add(id);
            return true;
          });
          const prepended = uniqueIncoming.filter((message: any) => {
            const meta = ((message.providerMeta || {}) as Record<string, unknown>);
            const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
            return String(messageMeta.kind || "").trim() === "summary_context_seed";
          });
          const appended = uniqueIncoming.filter((message: any) => {
            const meta = ((message.providerMeta || {}) as Record<string, unknown>);
            const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
            return String(messageMeta.kind || "").trim() !== "summary_context_seed";
          });
          const appendedOwnUser = appended.filter((message: any) => bindings.isLocalOwnUserMessage(message));
          const appendedOthers = appended.filter((message: any) => !bindings.isLocalOwnUserMessage(message));
          let nextMessages = [...currentMessages];
          if (prepended.length > 0) {
            nextMessages = [...prepended, ...nextMessages];
          }
          if (appendedOwnUser.length > 0) {
            let replacedOwnDraft = false;
            const remainingOwnIncoming = [...appendedOwnUser];
            nextMessages = nextMessages.flatMap((message: any) => {
              if (!replacedOwnDraft && bindings.isOptimisticOwnUserDraft(message)) {
                replacedOwnDraft = true;
                return [bindings.applyStableRenderIdFromDraft(remainingOwnIncoming.shift()!, message)];
              }
              return [message];
            });
            if (remainingOwnIncoming.length > 0) {
              nextMessages = bindings.mergeMessagesIntoTimeline(nextMessages, remainingOwnIncoming);
            }
          }
          if (appendedOthers.length > 0) {
            nextMessages = bindings.mergeMessagesIntoTimeline(nextMessages, appendedOthers);
          }
          nextMessages = bindings.reuseStableMessageReferences(nextMessages, bindings.allMessages.value);
          bindings.allMessages.value = nextMessages;
          bindings.foregroundTailLatestReady.value = true;
        }
        bindings.cacheConversationMessages(
          flushedConversationId || String(bindings.currentChatConversationId.value || "").trim(),
          bindings.allMessages.value,
        );
      },
  });
  const confirmPlan = useConfirmPlan({
      currentApiConfigId: bindings.currentForegroundApiConfigId,
      currentAgentId: bindings.currentForegroundAgentId,
      currentDepartmentId: bindings.currentForegroundDepartmentId,
      currentConversationId: bindings.currentChatConversationId,
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      compactingConversation: bindings.compactingConversation,
      setConversationPlanMode: bindings.setConversationPlanMode,
      clearForegroundRuntimeState: () => {
        chatFlowRef?.clearForegroundRuntimeState();
      },
      confirmPlanAndContinue: ({ conversationId, planMessageId, departmentId, agentId }) => invokeTauri<void>("confirm_plan_and_continue", {
        input: {
          conversationId,
          planMessageId,
          departmentId: departmentId || null,
          agentId: agentId || null,
        },
      }),
  });
  const rewindActions = useChatRewindActions({
      activeApiConfigId: bindings.currentForegroundApiConfigId,
      activeAgentId: bindings.currentForegroundAgentId,
      currentConversationId: bindings.currentChatConversationId,
      allMessages: bindings.allMessages,
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      compactingConversation: bindings.compactingConversation,
      chatErrorText: bindings.chatErrorText,
      chatInput: bindings.chatInput,
      selectedMentions: bindings.selectedChatMentions,
      clipboardImages: bindings.clipboardImages,
      queuedAttachmentNotices: bindings.queuedAttachmentNotices,
      deleteUnarchivedConversationFromArchives: bindings.deleteUnarchivedConversationFromArchives,
      sendChat: bindings.sendChatFromCurrentWindow,
      setStatusError: bindings.setStatusError,
      setChatErrorText: (text: string) => {
        bindings.chatErrorText.value = text;
      },
      removeBinaryPlaceholders: bindings.removeBinaryPlaceholders,
      messageText: bindings.messageText,
      extractMessageImages: bindings.extractMessageImages,
      extractMessageAttachmentFiles: bindings.extractMessageAttachmentFiles,
      requestRecallMode: bindings.requestRecallMode,
      requestCreateConversationBranchFromMessageConfirm: bindings.requestCreateConversationBranchFromMessageConfirm,
      createConversationBranchFromMessage: bindings.createConversationBranchFromMessage,
      branchingConversation: bindings.branchingConversation,
      refreshForegroundConversationAfterRewind: async (conversationId: string) => {
        const normalizedConversationId = String(conversationId || "").trim();
        if (!normalizedConversationId) return;
        chatFlowRef?.clearForegroundRuntimeState();
        const snapshot = await invokeTauri<any>("get_foreground_conversation_light_snapshot", {
          input: {
            conversationId: normalizedConversationId,
            agentId: null,
            limit: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
          },
        });
        bindings.applyConversationSnapshot(snapshot);
      },
  });

  chatFlowRef = chatFlow;
  const unregisterChatFlowRuntime = registerChatFlowRuntime({
    bindingId: chatFlow.bindingId,
    getConversationId: () => String(bindings.currentChatConversationId.value || "").trim(),
    flow: chatFlow,
  });
  onScopeDispose(() => {
    unregisterChatFlowRuntime();
    void chatFlow.unbindActiveConversationStream?.().catch(() => {});
  });

  return {
    chatFlow,
    handleConfirmPlan: confirmPlan.handleConfirmPlan,
    deleteUnarchivedConversation: rewindActions.deleteUnarchivedConversation,
    handleCreateConversationBranchFromTurn: rewindActions.handleCreateConversationBranchFromTurn,
    handleRecallTurn: rewindActions.handleRecallTurn,
    handleRegenerateTurn: rewindActions.handleRegenerateTurn,
  };
}
