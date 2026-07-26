import type {
  ApiConfigItem,
  ChatConversationOverviewItem,
  ChatMessage,
  ChatTodoItem,
  ConversationGoalState,
  IdeContextWorkspaceGroup,
} from "../../types/app";
import type { DepartmentPersonaOption } from "../shared/department-persona-options";

export type SidebarCreateDepartmentOption = DepartmentPersonaOption;

export type ConversationSummary = {
  conversationId: string;
  title: string;
  summaryTitle?: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount?: number;
  bodyMessageCount?: number;
  bodyTextLength?: number;
  unreadCount?: number;
  agentId?: string;
  departmentId?: string;
  departmentName?: string;
  runtimeState?: string;
  planModeEnabled?: boolean;
  detachedWindowOpen?: boolean;
  detachedWindowLabel?: string;
  isSystemNotificationConversation?: boolean;
  isMainConversation?: boolean;
  isActive?: boolean;
  isPinned?: boolean;
  pinIndex?: number;
  workspaceLabel?: string;
  workspaceRootPath?: string;
  currentTodo?: string;
  activeGoal?: ConversationGoalState | null;
  currentTodos?: ChatTodoItem[];
  state?: ChatConversationOverviewItem["state"];
  previewMessages?: Array<{
    messageId: string;
    role: ChatMessage["role"];
    speakerAgentId?: string;
    createdAt?: string;
    textPreview?: string;
    hasImage?: boolean;
    hasPdf?: boolean;
    hasAudio?: boolean;
    hasAttachment?: boolean;
  }>;
};

export type RemoteImContactConversationSummary = {
  contactId: string;
  conversationId: string;
  title: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount: number;
  channelId: string;
  channelName?: string;
  contactDisplayName: string;
  boundDepartmentId?: string;
  boundAgentId?: string;
  processingMode?: string;
  previewMessages?: ConversationSummary["previewMessages"];
};

export type OpenConversationResult = {
  conversationId: string;
  title: string;
  agentId?: string;
  departmentId?: string;
  messages: ChatMessage[];
  runtime?: SidebarConversationRuntimePayload | null;
  persona?: SidebarPersonaPayload;
  model?: SidebarModelPayload;
  currentTodos?: ChatTodoItem[];
  activeGoal?: ConversationGoalState | null;
};

export type SidebarWorkspacePermission = {
  access?: "read_only" | "approval" | "full_access" | "";
  workspaceName?: string;
  rootPath?: string;
};

export type SidebarClipboardImage = { mime: string; bytesBase64: string; previewDataUrl?: string };

export type SidebarQueuedAttachmentEntry = {
  id: string;
  fileName: string;
  path: string;
  mime: string;
  imageBytesBase64?: string;
  previewDataUrl?: string;
};

export type SidebarQueuedAttachmentNotice = {
  id: string;
  fileName: string;
  path: string;
  mime: string;
};

export type SidebarAttachmentPayload = {
  fileName: string;
  path: string;
  mime: string;
};

export type RewindConversationResult = {
  removedCount: number;
  remainingCount: number;
  recalledUserMessage?: ChatMessage;
};

export type RewindConversationPreviewResult = {
  conversationId: string;
  canUndoPatch: boolean;
  hint?: string | null;
};

export type BlockPageResult = {
  selectedBlockId: number;
  messages: ChatMessage[];
  hasPrevBlock: boolean;
  hasNextBlock: boolean;
};

export type CompactionPreviewResult = {
  conversationId: string;
  canCompact: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
};

export type SidebarPersonaPayload = {
  userAlias?: string;
  userAvatarUrl?: string;
  assistantName?: string;
  assistantAvatarUrl?: string;
  personaNameMap?: Record<string, string>;
  personaAvatarUrlMap?: Record<string, string>;
};

export type SidebarModelPayload = {
  conversationCallPrimaryApiConfigId?: string;
  preferredChatModelId?: string;
  toolReviewApiConfigId?: string;
  chatModelOptions?: ApiConfigItem[];
};

export type SidebarStreamCachePayload = {
  activationId?: string;
  requestId?: string;
  assistantText?: string;
  toolStatusText?: string;
  toolStatusState?: string;
  streamBlocks?: unknown[];
  updatedAt?: string;
  hasVisibleProgress?: boolean;
  persistedAssistantMessageId?: string;
};

export type SidebarConversationRuntimePayload = {
  runtimeState?: string;
  streamCache?: SidebarStreamCachePayload;
};

export type GoalMutationOutput = {
  conversationId: string;
  goal: ConversationGoalState;
};

export type SidebarAssistantDeltaPayload = {
  conversationId?: string;
  event?: {
    delta?: string;
    kind?: string;
    toolName?: string;
    toolCallId?: string;
    toolStatus?: string;
    toolArgs?: string;
    message?: string;
    streamCache?: SidebarStreamCachePayload;
  };
};

export type CreateConversationOptionsResult = {
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
  defaultAgentId?: string;
};

export type DiscoveryPayload = {
  chatUrl?: string;
  bridgeUrl?: string;
  url?: string;
  token?: string;
  workspaceRoots?: Array<{ path?: string; name?: string }>;
};

export type IdeContextQueryResult = {
  groups?: IdeContextWorkspaceGroup[];
  updatedAt?: string;
};
