<template>
  <div class="flex flex-wrap items-start gap-4 min-h-0 h-full pr-1">
    <!-- 左侧：渠道列表 -->
    <div class="self-start h-auto bg-base-100 rounded-box border border-base-300 min-w-88 flex-1 basis-104 flex flex-col overflow-hidden">
      <div class="flex items-center justify-between px-3 py-2 shrink-0">
        <span class="font-semibold text-sm">{{ t("config.remoteIm.title") }}</span>
        <div class="flex gap-1">
          <button class="btn btn-square btn-ghost" :title="t('config.remoteIm.addChannel')" @click="openAddChannelModal">
            <Plus class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
      <div v-if="channels.length === 0" class="text-xs italic opacity-60 py-4 text-center">
        {{ t("config.remoteIm.empty") }}
      </div>
      <div v-else class="flex-1 overflow-y-auto py-3">
        <div class="flex flex-wrap gap-3">
          <div
            v-for="ch in channels"
            :key="ch.id"
            class="w-48 max-w-full shrink-0 rounded-box border transition-colors"
            :class="selectedChannelId === ch.id ? 'border-primary bg-primary/8' : 'border-base-300 bg-base-200 hover:border-base-content/20'"
            @click="selectedChannelId = ch.id"
          >
            <div class="flex items-start justify-between gap-3 px-4 py-2">
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-semibold">
                  {{ ch.name || t('config.remoteIm.channelName') }}
                </div>
                <div class="mt-1 text-[11px] opacity-60 truncate">
                  {{ platformLabelText(ch.platform) }}
                </div>
              </div>
              <input
                type="checkbox"
                class="toggle toggle-primary toggle-sm mt-0.5 shrink-0"
                :checked="ch.enabled"
                :disabled="saving"
                @mousedown.stop
                @click.stop
                @change.stop="(e) => toggleChannelEnabled(ch, (e.target as HTMLInputElement).checked)"
              />
            </div>
            <div class="px-4 pb-2 flex items-center gap-2">
              <button
                class="btn btn-sm flex-1 border"
                :class="selectedChannelId === ch.id ? 'btn-primary border-primary' : 'border-base-300 bg-base-300 text-base-content hover:bg-base-content/10'"
                :title="t('config.remoteIm.channelDetails')"
                @click.stop="openChannelConfigModal(ch.id)"
              >
                {{ t("common.edit") }}
              </button>
              <button
                class="btn btn-sm btn-square border shrink-0"
                :class="selectedChannelId === ch.id ? 'btn-primary border-primary' : 'border-base-300 bg-base-300 text-base-content hover:bg-base-content/10'"
                :title="t('config.remoteIm.viewLogs')"
                @click.stop="openChannelLogsModalForChannel(ch.id)"
              >
                <ScrollText class="h-4 w-4" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 右侧：联系人列表 -->
    <div class="self-start h-auto bg-base-100 rounded-box border border-base-300 min-w-88 flex-1 basis-md flex flex-col overflow-hidden">
      <div class="flex items-center justify-between px-3 py-2 shrink-0">
        <span class="flex items-center gap-2 font-semibold text-sm">
          {{ t("config.remoteIm.contactsTitle") }}
          <span class="badge badge-ghost badge-xs">{{ currentChannelContacts.length }}</span>
        </span>
        <button class="btn btn-square btn-ghost" :title="t('common.refresh')" @click="refreshContacts">
          <RefreshCw class="h-3.5 w-3.5" :class="contactsLoading ? 'animate-spin' : ''" />
        </button>
      </div>
      <div v-if="contactsDisabledReason" class="px-3 pb-2 text-xs text-warning">
        {{ contactsDisabledReason }}
      </div>
      <ul class="w-full flex-1 overflow-y-auto px-0">
        <li v-if="contactsError" class="menu-title">
          <span class="text-xs text-error">{{ contactsError }}</span>
        </li>
        <li v-if="currentChannelContacts.length === 0" class="menu-title">
          <span class="text-xs italic opacity-60">{{ t("config.remoteIm.contactsEmpty") }}</span>
        </li>
        <template v-else>
          <template v-for="group in groupedContacts" :key="group.mode">
            <li class="menu-title text-base-content">
              <span class="text-sm font-bold">{{ group.label }}（{{ group.items.length }}）</span>
            </li>
            <li v-for="item in group.items" :key="item.id" class="border-b border-base-200 last:border-b-0">
            <div class="flex items-start gap-2 px-3 py-2">
                <span class="badge shrink-0" :class="item.remoteContactType === 'group' ? 'badge-secondary' : 'badge-primary'">{{ item.remoteContactType === "group" ? t("config.remoteIm.group") : t("config.remoteIm.private") }}</span>
                <div class="flex-1 min-w-0">
                  <div class="truncate font-semibold">
                    <span class="font-normal opacity-70">[{{ contactDepartmentLabel(item) }}]</span>
                    {{ " " }}
                    {{ contactSafeDisplayName(item) }}
                    <span class="text-xs font-normal opacity-50">（{{ contactSecondaryText(item) }}）</span>
                  </div>
                  <div class="mt-1 flex flex-wrap gap-1 text-[10px]">
                    <span
                      class="badge badge-info badge-xs"
                      :title="processingModeHintText(item)"
                    >
                      {{ contactProcessingModeLabel(item) }}
                    </span>
                    <span
                      class="badge badge-xs"
                      :class="normalizeResponseStrategy(item.responseStrategy) === 'smart_judge' ? 'badge-accent' : 'badge-ghost'"
                      :title="contactResponseStrategyHintText(item)"
                    >
                      {{ normalizeResponseStrategy(item.responseStrategy) === "smart_judge" ? t("config.remoteIm.responseStrategySmart") : t("config.remoteIm.responseStrategyAlways") }}
                    </span>
                    <span
                      v-if="item.allowSendFiles"
                      class="badge badge-xs badge-warning"
                      :title="t('config.remoteIm.allowSendFiles')"
                    >
                      {{ t("config.remoteIm.filesShort") }}
                    </span>
                  </div>
                </div>
                <div class="flex items-center gap-1">
                  <input
                    type="checkbox"
                    class="toggle toggle-sm"
                    :class="contactCommunicationToggleClass(item)"
                    :checked="contactCommunicationToggleEnabled(item)"
                    :disabled="contactsDisabled"
                    :title="`${t('config.remoteIm.allowReceive')} / ${t('config.remoteIm.allowSend')}`"
                    @click.stop
                    @change="toggleContactCommunication(item, ($event.target as HTMLInputElement).checked)"
                  />
                  <div v-if="contactNeedsQuickModel(item) && !props.config.toolReviewApiConfigId" class="dropdown dropdown-end">
                    <div tabindex="0" role="button" class="btn btn-ghost btn-square btn-sm text-error hover:bg-error hover:text-error-content">
                      <AlertTriangle class="h-4 w-4" />
                    </div>
                    <div tabindex="0" class="dropdown-content card card-sm bg-base-100 border border-error/30 shadow-lg z-10 w-64">
                      <div class="card-body p-3">
                        <p class="text-error text-xs">{{ t('config.remoteIm.quickModelMissingHint') }}</p>
                      </div>
                    </div>
                  </div>
                  <button
                    class="btn btn-ghost btn-square btn-sm hover:bg-base-300"
                    :title="t('config.remoteIm.viewLogs')"
                    @click.stop="openContactLogsModal(item.id)"
                  >
                    <ScrollText class="h-4 w-4" />
                  </button>
                  <button
                    class="btn btn-ghost btn-square btn-sm hover:bg-base-300"
                    :title="t('config.remoteIm.channelDetails')"
                    :disabled="contactsDisabled"
                    @click.stop="openContactConfigModal(item.id)"
                  >
                    <Settings class="h-4 w-4" />
                  </button>
                </div>
            </div>
          </li>
          </template>
        </template>
      </ul>
    </div>

    <div class="modal z-90" :class="{ 'modal-open': addChannelModalOpen }" @click.self="closeAddChannelModal">
      <div class="modal-box max-w-md">
        <div class="flex items-center justify-between">
          <div class="font-semibold text-lg">{{ t("config.remoteIm.addChannel") }}</div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeAddChannelModal">
            <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div class="mt-3 text-sm opacity-70">{{ t("config.remoteIm.choosePlatform") }}</div>
        <div class="mt-4 grid grid-cols-1 gap-2">
          <button
            v-for="option in channelPlatformOptions"
            :key="option.platform"
            class="btn btn-outline justify-start h-auto min-h-0 py-3 px-4 normal-case"
            @click="addChannel(option.platform)"
          >
            <span class="font-medium">{{ option.label }}</span>
          </button>
        </div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeAddChannelModal">{{ t("common.cancel") }}</button>
        </div>
      </div>
    </div>

    <div class="modal z-90" :class="{ 'modal-open': channelLogsModalOpen }" @click.self="closeChannelLogsModal">
      <div class="modal-box max-w-4xl">
        <div class="flex items-center justify-between">
          <div class="font-semibold">
            {{ t("config.remoteIm.channelLogs") }} · {{ selectedChannel?.name || "-" }}
          </div>
          <div class="flex items-center gap-2">
            <button class="btn btn-sm btn-ghost" :title="t('common.refresh')" @click="refreshChannelLogs">
              <RefreshCw class="h-4 w-4" :class="channelLogsLoading ? 'animate-spin' : ''" />
            </button>
            <button class="btn btn-sm" @click="closeChannelLogsModal">{{ t("common.close") }}</button>
          </div>
        </div>
        <div class="mt-3 max-h-[60vh] overflow-y-auto">
          <div v-if="channelLogs.length === 0" class="opacity-60 italic text-xs">{{ t("config.remoteIm.noLogs") }}</div>
          <pre v-else class="bg-base-200 rounded-box p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all m-0"><template v-for="(log, idx) in channelLogs" :key="idx"><span :class="log.level === 'error' ? 'text-error' : log.level === 'warn' ? 'text-warning' : ''"><span class="opacity-50">{{ formatLogTime(log.timestamp) }}</span> {{ log.message }}</span>{{ '\n' }}</template></pre>
        </div>
      </div>
    </div>

    <div class="modal z-90" :class="{ 'modal-open': contactLogsModalOpen }" @click.self="closeContactLogsModal">
      <div class="modal-box max-w-4xl">
        <div class="flex items-center justify-between">
          <div class="font-semibold">
            联系人日志 · {{ contactLogsTitle }}
          </div>
          <div class="flex items-center gap-2">
            <button class="btn btn-sm btn-ghost" :title="t('common.refresh')" @click="refreshContactLogs">
              <RefreshCw class="h-4 w-4" :class="contactLogsLoading ? 'animate-spin' : ''" />
            </button>
            <button class="btn btn-sm" @click="closeContactLogsModal">{{ t("common.close") }}</button>
          </div>
        </div>
        <div class="mt-3 max-h-[60vh] overflow-y-auto">
          <div v-if="contactLogs.length === 0" class="opacity-60 italic text-xs">暂无该联系人的相关日志</div>
          <pre v-else class="bg-base-200 rounded-box p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all m-0"><template v-for="(line, idx) in contactLogDisplayLines" :key="`${idx}-${line}`"><span>{{ line }}</span>{{ '\n' }}</template></pre>
        </div>
      </div>
    </div>

    <!-- 渠道配置模态框 -->
    <div class="modal z-80" :class="{ 'modal-open': channelConfigModalOpen }" @click.self="closeChannelConfigModal">
      <div class="modal-box max-w-3xl max-h-[80vh] overflow-hidden flex flex-col">
        <div class="flex items-center justify-between shrink-0">
          <div class="font-semibold text-lg">
            {{ t("config.remoteIm.channelDetails") }} · {{ selectedChannel?.name || "-" }}
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeChannelConfigModal">
            <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div v-if="selectedChannel" class="flex-1 min-h-0 overflow-hidden flex flex-col mt-4">
          <!-- 头部 -->
          <div class="flex items-center justify-between px-3 py-2 shrink-0">
            <div></div>
          </div>

          <!-- 状态栏 -->
          <div class="px-3 pb-2 shrink-0">
            <div class="rounded-box border border-base-300 bg-base-200/60 px-3 py-2 flex items-center justify-between gap-3">
              <div class="flex items-center gap-2 min-w-0">
                <span
                  class="size-2 rounded-full shrink-0"
                  :class="selectedChannel.platform === 'onebot_v11'
                    ? (channelRuntimeStates[selectedChannel.id]?.connected ? 'bg-success' : (selectedChannel.enabled ? 'bg-warning' : 'bg-base-300'))
                    : (selectedChannel.platform === 'dingtalk'
                      ? (channelRuntimeStates[selectedChannel.id]?.connected ? 'bg-success' : (selectedChannel.enabled ? 'bg-warning' : 'bg-base-300'))
                      : ((selectedChannel.platform === 'feishu')
                        ? (selectedChannel.enabled ? 'bg-warning' : 'bg-base-300')
                        : (selectedChannel.enabled ? 'bg-success' : 'bg-base-300')))"
                ></span>
                <span class="text-xs font-medium">{{ t("config.remoteIm.connectionStatus") }}</span>
                <span class="text-xs opacity-80 truncate">{{ channelStatusPreview(selectedChannel!) }}</span>
              </div>
              <div class="flex items-center gap-2">
                <button class="btn btn-xs btn-ghost" @click="openChannelLogsModal">
                  {{ t("config.remoteIm.viewLogs") }}
                </button>
              </div>
            </div>
          </div>

          <!-- 内容滚动区 -->
          <div class="flex-1 overflow-y-auto px-3 text-xs pb-4">
              <!-- 渠道名称 -->
              <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                <span>{{ t("config.remoteIm.channelName") }}</span>
                <input v-model="selectedChannel.name" class="input input-bordered input-sm w-48" :placeholder="t('config.remoteIm.channelName')" />
              </div>
              <!-- 平台 -->
              <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                <span>{{ t("config.remoteIm.platform") }}</span>
                <select v-model="selectedChannel.platform" class="select select-bordered select-sm w-48">
                  <option value="onebot_v11">{{ t("config.remoteIm.platformOptions.onebotV11") }}</option>
                  <option value="feishu">{{ t("config.remoteIm.platformOptions.feishu") }}</option>
                  <option value="dingtalk">{{ t("config.remoteIm.platformOptions.dingtalk") }}</option>
                  <option value="weixin_oc">个人微信</option>
                </select>
              </div>

              <!-- OneBot v11 凭证配置 -->
              <template v-if="selectedChannel.platform === 'onebot_v11'">
                <div class="border-b-base-content/5 flex flex-col gap-2 border-b border-dashed py-2 mt-2">
                  <span class="font-semibold">{{ t("config.remoteIm.napcatConfig") }}</span>
                </div>
                <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                  <span>{{ t("config.remoteIm.wsHost") }}</span>
                  <input v-model="napcatCredentials.wsHost" class="input input-bordered input-sm w-32" placeholder="0.0.0.0" />
                </div>
                <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                  <span>{{ t("config.remoteIm.wsPort") }}</span>
                  <input v-model.number="napcatCredentials.wsPort" type="number" class="input input-bordered input-sm w-32" placeholder="6199" />
                </div>
                <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                  <span>{{ t("config.remoteIm.wsToken") }}</span>
                  <input v-model="napcatCredentials.wsToken" class="input input-bordered input-sm w-32" :placeholder="t('config.remoteIm.wsTokenPlaceholder')" />
                </div>
              </template>

              <!-- 钉钉凭证 -->
              <template v-else-if="selectedChannel.platform === 'dingtalk'">
                <div class="border-b-base-content/5 flex flex-col gap-2 border-b border-dashed py-2 mt-2">
                  <span class="font-semibold">{{ t("config.remoteIm.dingtalkCredentials") }}</span>
                </div>
                <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                  <span>{{ t("config.remoteIm.dingtalkClientId") }}</span>
                  <input
                    v-model="dingtalkCredentials.clientId"
                    class="input input-bordered input-sm w-72"
                    placeholder="dingxxxxxxxxxxxxxxxx"
                  />
                </div>
                <div class="border-b-base-content/5 flex items-center justify-between gap-2 border-b border-dashed py-2">
                  <span>{{ t("config.remoteIm.dingtalkClientSecret") }}</span>
                  <div class="flex items-center gap-2">
                    <input
                      v-model="dingtalkCredentials.clientSecret"
                      :type="showDingtalkSecret ? 'text' : 'password'"
                      class="input input-bordered input-sm w-72"
                      placeholder="xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
                    />
                    <button
                      class="btn btn-xs btn-ghost"
                      type="button"
                      @click="showDingtalkSecret = !showDingtalkSecret"
                    >
                      {{ showDingtalkSecret ? "隐藏" : "显示" }}
                    </button>
                  </div>
                </div>
              </template>

              <template v-else-if="selectedChannel.platform === 'weixin_oc'">
                <div class="border-b-base-content/5 flex flex-col gap-2 border-b border-dashed py-2 mt-2">
                  <span class="font-semibold">个人微信扫码登录</span>
                </div>
                <div class="border-b-base-content/5 flex items-start justify-between gap-2 border-b border-dashed py-2">
                  <div class="flex flex-col gap-1">
                    <span>登录状态</span>
                    <span class="opacity-70 break-all">{{ weixinStatusText }}</span>
                    <span v-if="weixinStatusMessage" class="opacity-60 break-all">{{ weixinStatusMessage }}</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <button class="btn btn-primary" :disabled="weixinLoginBusy" @click="onWeixinLoginButtonClick">
                      {{ weixinLoginBusy ? "处理中" : (isWeixinLoggedIn ? "退出登录并重新扫码" : "扫码登录") }}
                    </button>
                  </div>
                </div>
                <div v-if="isWeixinLoggedIn" class="border-b-base-content/5 flex items-center gap-2 border-b border-dashed py-2 text-success">
                  <span class="font-semibold">已登录，可直接使用</span>
                </div>
                <div v-else-if="weixinLoginState.qrcodeImgContent" class="border-b-base-content/5 flex flex-col gap-2 border-b border-dashed py-2">
                  <span class="font-semibold">扫码二维码</span>
                  <img :src="weixinQrImageSrc" alt="weixin login qr" class="w-48 h-48 rounded-box border border-base-300 object-contain bg-white p-2" />
                </div>
              </template>

              <!-- 飞书凭证 JSON -->
              <template v-else>
                <div class="border-b-base-content/5 flex flex-col gap-2 border-b border-dashed py-2 mt-2">
                  <span class="font-semibold">{{ t("config.remoteIm.credentialsJson") }}</span>
                  <textarea
                    v-model="credentialDrafts[selectedChannel.id]"
                    class="textarea textarea-bordered w-full min-h-20 font-mono"
                    spellcheck="false"
                    @blur="syncCredentialJson(selectedChannel)"
                  />
                </div>
              </template>

            <!-- 连接状态区域 (仅 OneBot v11) -->
            <template v-if="selectedChannel.platform === 'onebot_v11'">
              <div class="border-t border-base-300 mt-2 pt-2">
                <div class="flex items-center justify-between">
                  <span class="font-semibold">{{ t("config.remoteIm.connectionStatus") }}</span>
                  <button class="btn btn-square btn-ghost" :title="t('common.refresh')" @click="refreshChannelStatus">
                    <RefreshCw class="h-3.5 w-3.5" />
                  </button>
                </div>
                <div class="mt-2 flex items-center gap-2">
                  <span class="size-2 rounded-full" :class="channelStatus?.connected ? 'bg-success' : 'bg-base-300'"></span>
                  <span class="text-xs">
                    {{ channelStatus?.connected
                      ? `${t("config.remoteIm.connected")} (${channelStatus.peerAddr})`
                      : channelStatus?.statusText === "binding_retry"
                        ? (channelStatus.lastError || "固定端口被占用，正在重试")
                      : channelStatus?.listenAddr
                        ? t("config.remoteIm.waitingForConnection")
                        : t("config.remoteIm.serverNotStarted") }}
                  </span>
                </div>
              </div>

            </template>
          </div>

          <!-- 底部操作区（固定在滚动区外） -->
          <div class="px-3 py-2 shrink-0 border-t border-base-300 flex items-center justify-between">
            <button
              class="btn btn-ghost"
              :title="t('common.delete')"
              @click="removeChannelById(selectedChannel.id); closeChannelConfigModal()"
            >
              <Trash2 class="h-3.5 w-3.5" />
              {{ t("common.delete") }}
            </button>
            <div class="flex items-center gap-2">
              <button
                v-if="selectedChannel.platform === 'onebot_v11'"
                class="btn btn-ghost"
                :title="t('common.reset')"
                @click="resetNapcatCredentials"
              >
                <RotateCcw class="h-3.5 w-3.5" />
                {{ t("common.reset") }}
              </button>
              <button
                class="btn"
                :class="channelDirty ? 'btn-primary' : 'btn-ghost'"
                :disabled="!channelDirty || saving"
                @click="saveChannels"
              >
                <Save v-if="!saving" class="h-3.5 w-3.5" />
                <span v-else class="loading loading-spinner loading-xs"></span>
                {{ t("common.save") }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 联系人配置模态框 -->
    <div class="modal z-80" :class="{ 'modal-open': contactConfigModalOpen }" @click.self="closeContactConfigModal">
      <div class="modal-box max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        <div class="flex items-center justify-between shrink-0">
          <div class="font-semibold text-lg flex items-center gap-2">
            <span class="inline-flex h-8 w-8 items-center justify-center rounded-full bg-[#07c160] text-white text-sm font-bold">微</span>
            <span>{{ t("config.remoteIm.contactSettingsTitle", { name: contactModalTitle }) }}</span>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeContactConfigModal">
            <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div v-if="selectedContact && contactDraft" class="mt-4 flex-1 min-h-0 overflow-hidden flex flex-col">
          <div class="flex-1 overflow-y-auto">
            <ul class="list gap-2">
              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.processingDepartment") }}</div>
                <div class="flex w-64 flex-col gap-1">
                  <select
                    class="select select-bordered select-sm w-full"
                    v-model="contactDraft.boundDepartmentId"
                  >
                    <option value="">{{ t("config.department.assistantBadge") }}</option>
                    <option v-for="dept in remoteImDepartmentOptions" :key="dept.id" :value="dept.id">{{ dept.label }}</option>
                  </select>
                  <span class="text-[11px] opacity-60">{{ contactDraftRoutingHint }}</span>
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.processingMode") }}</div>
                <div class="flex w-64 flex-col gap-1">
                  <select
                    class="select select-bordered select-sm w-full"
                    v-model="contactDraft.processingMode"
                  >
                    <option value="continuous">{{ t("config.remoteIm.processingModeContinuous") }}</option>
                    <option value="qa">{{ t("config.remoteIm.processingModeQa") }}</option>
                  </select>
                  <span class="text-[11px] opacity-60">{{ contactDraftProcessingHint }}</span>
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.activateMode") }}</div>
                <div class="flex w-64 flex-col gap-2">
                  <select
                    class="select select-bordered select-sm w-full"
                    v-model="contactDraft.activationMode"
                  >
                    <option value="always">{{ t("config.remoteIm.activateModeAlways") }}</option>
                    <option value="never">{{ t("config.remoteIm.activateModeNever") }}</option>
                    <option value="keyword">{{ t("config.remoteIm.activateModeKeyword") }}</option>
                  </select>
                  <span class="text-[11px] opacity-60">{{ contactDraftActivationHint }}</span>
                  <input
                    v-if="contactDraft.activationMode === 'keyword'"
                    type="text"
                    class="input input-bordered input-sm w-full"
                    :placeholder="t('config.remoteIm.activateKeywordsPlaceholder')"
                    v-model="contactDraft.activationKeywordsText"
                  />
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.muteKeywords") }}</div>
                <div class="flex w-64 flex-col gap-2">
                  <input
                    type="text"
                    class="input input-bordered input-sm w-full"
                    :placeholder="t('config.remoteIm.muteKeywordsPlaceholder')"
                    v-model="contactDraft.muteKeywordsText"
                  />
                  <span class="text-[11px] opacity-60">
                    {{ t("config.remoteIm.muteKeywordsHint") }}
                  </span>
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.unmuteKeywords") }}</div>
                <div class="flex w-64 flex-col gap-2">
                  <input
                    type="text"
                    class="input input-bordered input-sm w-full"
                    :placeholder="t('config.remoteIm.unmuteKeywordsPlaceholder')"
                    v-model="contactDraft.unmuteKeywordsText"
                  />
                  <span class="text-[11px] opacity-60">
                    {{ t("config.remoteIm.unmuteKeywordsHint") }}
                  </span>
                </div>
              </li>

              <li class="list-row flex items-center justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.muteDuration") }}</div>
                <div class="flex w-64 items-center gap-2">
                  <input
                    type="number"
                    class="input input-bordered input-sm w-20"
                    v-model.number="contactDraft.muteDurationSeconds"
                    min="0"
                  />
                  <span class="opacity-60">{{ t("config.remoteIm.seconds") }}</span>
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.responseStrategy") }}</div>
                <div class="flex w-64 flex-col gap-2">
                  <select
                    class="select select-bordered select-sm w-full"
                    v-model="contactDraft.responseStrategy"
                  >
                    <option value="always_reply">{{ t("config.remoteIm.responseStrategyAlways") }}</option>
                    <option value="smart_judge">{{ t("config.remoteIm.responseStrategySmart") }}</option>
                  </select>
                  <span class="text-[11px] opacity-60">{{ contactDraftResponseStrategyHint }}</span>
                </div>
              </li>

              <li class="list-row flex items-start justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.responseGuidance") }}</div>
                <div class="flex w-64 flex-col gap-2">
                  <textarea
                    class="textarea textarea-bordered textarea-sm min-h-28 w-full"
                    v-model="contactDraft.responseGuidance"
                    :placeholder="t('config.remoteIm.responseGuidancePlaceholder')"
                  />
                  <span class="text-[11px] opacity-60">
                    {{ t("config.remoteIm.responseGuidanceHint") }}
                  </span>
                </div>
              </li>

              <li class="list-row flex items-center justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.patienceExit") }}</div>
                <div class="flex w-64 items-center gap-2">
                  <input
                    type="number"
                    class="input input-bordered input-sm w-20"
                    v-model.number="contactDraft.patienceSeconds"
                    min="0"
                  />
                  <span class="opacity-60">{{ t("config.remoteIm.seconds") }}</span>
                </div>
              </li>

              <li class="list-row flex items-center justify-between gap-3">
                <div class="font-medium">{{ t("config.remoteIm.allowSendFiles") }}</div>
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  v-model="contactDraft.allowSendFiles"
                />
              </li>
            </ul>

              <!-- 工作目录配置 -->
              <li class="list-row flex flex-col gap-2 pt-3 mt-2 border-t border-base-300">
                <div class="flex items-center justify-between">
                  <div class="font-medium">{{ t("config.remoteIm.workspace") }}</div>
                  <span class="text-[11px] opacity-50">{{ t("config.remoteIm.systemWorkspaceReadonly") }}</span>
                </div>
                <div
                  v-if="contactDraft.shellWorkspaces.length === 0"
                  class="rounded-box border border-dashed border-base-300 bg-base-200/20 px-3 py-4 text-center text-xs opacity-60"
                >
                  {{ t("config.remoteIm.noCustomWorkspace") }}
                </div>
                <div v-else class="divide-y divide-base-300">
                  <div
                    v-for="ws in contactDraft.shellWorkspaces"
                    :key="ws.id"
                    class="py-2 text-left"
                    :title="ws.path"
                  >
                    <div class="flex items-center gap-3">
                      <div class="min-w-0 flex-1 text-left">
                        <div class="flex flex-wrap items-center gap-2">
                          <span class="inline-block w-40 truncate font-medium align-middle" :title="ws.path">{{ ws.name }}</span>
                          <span v-if="ws.level === 'main'" class="badge badge-primary">
                            {{ t("config.tools.workspaceLevelMain") }}
                          </span>
                          <span class="badge" :class="ws.access === 'full_access' ? 'badge-success' : ws.access === 'approval' ? 'badge-warning' : 'badge-ghost'">
                            {{ ws.access === 'full_access' ? t("config.tools.workspaceAccessFullAccess") : ws.access === 'approval' ? t("config.tools.workspaceAccessApproval") : t("config.tools.workspaceAccessReadOnly") }}
                          </span>
                        </div>
                      </div>
                      <div class="flex shrink-0 flex-wrap items-center justify-end gap-2">
                        <button
                          v-if="ws.level !== 'main'"
                          class="btn btn-xs btn-ghost"
                          type="button"
                          :title="t('config.tools.setWorkspaceAsMain')"
                          @click="setContactWorkspaceMain(ws.id)"
                        >
                          <SquareTerminal class="h-3.5 w-3.5" />
                        </button>
                        <button
                          v-else
                          class="btn btn-xs btn-primary pointer-events-none opacity-100"
                          type="button"
                          aria-disabled="true"
                          tabindex="-1"
                          :title="t('config.tools.currentMainWorkspace')"
                        >
                          <SquareTerminal class="h-3.5 w-3.5" />
                        </button>
                        <select
                          class="select select-sm select-bordered w-32"
                          :value="ws.access"
                          @change="updateContactWorkspaceAccess(ws.id, ($event.target as HTMLSelectElement).value as ShellWorkspace['access'])"
                        >
                          <option value="full_access">{{ t("config.tools.workspaceAccessFullAccess") }}</option>
                          <option value="approval">{{ t("config.tools.workspaceAccessApproval") }}</option>
                          <option value="read_only">{{ t("config.tools.workspaceAccessReadOnly") }}</option>
                        </select>
                        <button
                          class="btn btn-sm btn-ghost text-error"
                          type="button"
                          :title="t('config.tools.delete')"
                          @click="removeContactWorkspace(ws.id)"
                        >
                          <Trash2 class="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
                <div class="flex justify-end">
                  <button class="btn btn-sm" type="button" @click="addContactWorkspace">
                    {{ t("config.tools.addWorkspace") }}
                  </button>
                </div>
              </li>

          <div class="mt-2 text-[11px] opacity-60 leading-5">
            {{ t("config.remoteIm.contactConversationHint") }}
          </div>
          </div>
          <div class="mt-3 pt-3 border-t border-base-300 flex items-center justify-between gap-2 shrink-0">
            <button
              class="btn btn-ghost text-error"
              :disabled="contactsDisabled || contactSaving || contactDeleting || !selectedContact"
              @click="selectedContact && deleteContact(selectedContact)"
            >
              <Trash2 class="h-3.5 w-3.5" />
              {{ t("common.delete") }}
            </button>
            <div class="flex items-center gap-2">
            <button class="btn btn-ghost" :disabled="!contactDraftDirty || contactSaving" @click="resetContactDraft">
              <RotateCcw class="h-3.5 w-3.5" />
              {{ t("common.reset") }}
            </button>
            <button class="btn btn-primary" :disabled="contactsDisabled || !contactDraftDirty || contactSaving || contactDeleting" @click="saveContactDraft">
              <Save v-if="!contactSaving" class="h-3.5 w-3.5" />
              <span v-else class="loading loading-spinner loading-xs"></span>
              {{ t("common.save") }}
            </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { AlertTriangle, Plus, RefreshCw, RotateCcw, Save, ScrollText, Settings, SquareTerminal, Trash2 } from "@lucide/vue";
import { invokeTauri } from "../../../../services/tauri-api";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppConfig, DepartmentConfig, RemoteImChannelConfig, RemoteImContact, RemoteImPlatform, ShellWorkspace } from "../../../../types/app";
import type { ChannelConnectionStatus, ChannelLogEntry, WeixinLoginStatus } from "./remote-im/types";
import {
  contactCommunicationToggleClass,
  contactCommunicationToggleEnabled,
  formatLogTime,
  normalizeActivationMode,
  normalizeProcessingMode,
  normalizeResponseStrategy,
  parseActivationKeywords,
  parseKeywordList,
} from "./remote-im/helpers";

const props = defineProps<{
  config: AppConfig;
  saveConfigAction: () => Promise<boolean> | boolean;
  setStatusAction: (text: string) => void;
}>();

const { t } = useI18n();
const WEIXIN_OC_BOT_TYPE = "3";
const WEIXIN_OC_QR_POLL_INTERVAL = 1;
const WEIXIN_OC_LONG_POLL_TIMEOUT_MS = 35000;
const WEIXIN_OC_API_TIMEOUT_MS = 15000;
const saving = ref(false);
const contactsLoading = ref(false);
const contactsError = ref("");
const contacts = ref<RemoteImContact[]>([]);
const credentialDrafts = ref<Record<string, string>>({});
const napcatCredentials = ref({ wsHost: "0.0.0.0", wsPort: 6199, wsToken: "" });
const dingtalkCredentials = ref({ clientId: "", clientSecret: "" });
const weixinCredentials = ref({
  baseUrl: "https://ilinkai.weixin.qq.com",
  botType: WEIXIN_OC_BOT_TYPE,
  qrPollInterval: WEIXIN_OC_QR_POLL_INTERVAL,
  longPollTimeoutMs: WEIXIN_OC_LONG_POLL_TIMEOUT_MS,
  apiTimeoutMs: WEIXIN_OC_API_TIMEOUT_MS,
});
const showDingtalkSecret = ref(false);
const suppressCredentialSync = ref(false);
const newWorkspacePath = ref("");
const selectedChannelId = ref<string>("");
const channels = computed(() => props.config.remoteImChannels || []);
const channelStatus = ref<ChannelConnectionStatus | null>(null);
const channelLogs = ref<ChannelLogEntry[]>([]);
const channelLogsModalOpen = ref(false);
const channelLogsLoading = ref(false);
const contactLogs = ref<ChannelLogEntry[]>([]);
const contactLogsModalOpen = ref(false);
const contactLogsLoading = ref(false);
const contactLogsContactId = ref("");
const addChannelModalOpen = ref(false);
const channelConfigModalOpen = ref(false);
const contactConfigModalOpen = ref(false);
const selectedContactId = ref<string>("");
const contactSaving = ref(false);
const contactDeleting = ref(false);
const channelRuntimeStates = ref<Record<string, ChannelConnectionStatus | null>>({});
const weixinLoginStates = ref<Record<string, WeixinLoginStatus | null>>({});
const weixinLoginBusy = ref(false);
let weixinLoginPollTimer: ReturnType<typeof setInterval> | null = null;
let channelStatusTimer: ReturnType<typeof setInterval> | null = null;

const selectedChannel = computed(() =>
  channels.value.find((ch) => ch.id === selectedChannelId.value) ?? null,
);

const weixinLoginState = computed(() => {
  const channelId = selectedChannel.value?.id || "";
  return weixinLoginStates.value[channelId] || {
    channelId,
    connected: false,
    status: "",
    message: "",
    sessionKey: "",
    qrcode: "",
    qrcodeImgContent: "",
    accountId: "",
    userId: "",
    baseUrl: "",
    lastError: "",
  };
});

function looksLikeBase64(value: string): boolean {
  if (!value || value.length < 64) return false;
  return /^[A-Za-z0-9+/=]+$/.test(value);
}

const weixinQrImageSrc = computed(() => {
  const raw = String(weixinLoginState.value.qrcodeImgContent || "").trim();
  if (!raw) return "";
  if (raw.startsWith("data:image/")) return raw;
  if (/^https?:\/\//i.test(raw)) {
    return `https://api.qrserver.com/v1/create-qr-code/?size=384x384&margin=0&data=${encodeURIComponent(raw)}`;
  }
  if (looksLikeBase64(raw)) {
    return `data:image/png;base64,${raw}`;
  }
  return raw;
});
const persistedWeixinCredentials = computed(() => {
  const creds = selectedChannel.value?.credentials;
  if (!creds || typeof creds !== "object") {
    return { token: "", accountId: "", userId: "" };
  }
  const record = creds as Record<string, unknown>;
  return {
    token: String(record.token || "").trim(),
    accountId: String(record.accountId || "").trim(),
    userId: String(record.userId || "").trim(),
  };
});
const weixinRuntimeStatus = computed(() =>
  selectedChannel.value ? channelRuntimeStates.value[selectedChannel.value.id] ?? null : null,
);
const weixinStatusText = computed(() => {
  if (weixinRuntimeStatus.value?.connected) return "已连接";
  if (isWeixinLoggedIn.value) return "已登录";
  const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
  if (status === "wait" || status === "scanned" || status === "scaned") return "等待扫码确认";
  if (status === "need_login" || status === "idle") return "待扫码登录";
  if (status === "confirmed" || status === "logged_in") return "已登录";
  return "待扫码登录";
});
const weixinStatusMessage = computed(() => {
  if (weixinRuntimeStatus.value?.connected) {
    return "凭证已保存";
  }
  if (isWeixinLoggedIn.value) {
    return "凭证已保存";
  }
  const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
  if (status === "wait" || status === "scanned" || status === "scaned") {
    return "请在微信中确认登录";
  }
  const errorMessage = String(weixinLoginState.value.lastError || "").trim();
  return errorMessage || "";
});
const isWeixinLoggedIn = computed(() => {
  const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
  if (weixinLoginState.value.connected) return true;
  if (weixinRuntimeStatus.value?.connected) return true;
  if (status === "confirmed" || status === "logged_in") return true;
  if (!!String(weixinLoginState.value.accountId || "").trim()) return true;
  if (!!persistedWeixinCredentials.value.token) return true;
  return !!persistedWeixinCredentials.value.accountId;
});
const channelPlatformOptions = computed<Array<{ platform: RemoteImPlatform; label: string }>>(() => [
  { platform: "onebot_v11", label: t("config.remoteIm.platformOptions.onebotV11") },
  { platform: "feishu", label: t("config.remoteIm.platformOptions.feishu") },
  { platform: "dingtalk", label: t("config.remoteIm.platformOptions.dingtalk") },
  { platform: "weixin_oc", label: t("config.remoteIm.platformOptions.weixinOc") },
]);

const channelSnapshot = computed(() => {
  const ch = selectedChannel.value;
  if (!ch) return "";
  const credStr = JSON.stringify(ch.credentials || {}, Object.keys(ch.credentials || {}).sort());
  return JSON.stringify({
    name: ch.name,
    platform: ch.platform,
    enabled: ch.enabled,
    activateAssistant: ch.activateAssistant,
    receiveFiles: ch.receiveFiles,
    streamingSend: ch.streamingSend,
    showToolCalls: ch.showToolCalls,
    credentials: credStr,
  });
});
const lastSavedChannelSnapshot = ref(channelSnapshot.value);
const channelDirty = computed(() => channelSnapshot.value !== lastSavedChannelSnapshot.value);

const currentChannelContacts = computed(() => {
  if (!selectedChannelId.value) return [];
  return contacts.value.filter((c) => c.channelId === selectedChannelId.value);
});
const contactsDisabledReason = computed(() => {
  const channel = selectedChannel.value;
  if (!channel) return "";
  if (!channel.enabled) return "当前渠道未启用，联系人列表不可操作。";
  if (channel.platform === "feishu") return "";
  if (channel.platform === "onebot_v11" || channel.platform === "dingtalk" || channel.platform === "weixin_oc") {
    const status = channelRuntimeStates.value[channel.id];
    if (status?.connected) return "";
    if (channel.platform === "onebot_v11" && status?.statusText === "binding_retry") {
      return status.lastError || "OneBot 固定端口被占用，正在按原端口重试绑定。";
    }
    if (channel.platform === "onebot_v11" && status?.statusText === "binding") {
      return "OneBot 正在绑定固定端口，连接成功前联系人列表不可操作。";
    }
    return "渠道尚未真正连接，连接成功前联系人列表不可操作。";
  }
  return "";
});
const contactsDisabled = computed(() => !!contactsDisabledReason.value);

type ContactGroup = { mode: "always" | "keyword" | "never"; label: string; items: typeof currentChannelContacts.value };
const groupedContacts = computed<ContactGroup[]>(() => {
  const all = currentChannelContacts.value;
  const groups: { mode: ContactGroup["mode"]; label: string; items: typeof all }[] = [
    { mode: "always", label: t("config.remoteIm.activateModeAlways"), items: [] },
    { mode: "keyword", label: t("config.remoteIm.activateModeKeyword"), items: [] },
    { mode: "never", label: t("config.remoteIm.activateModeNever"), items: [] },
  ];
  for (const c of all) {
    const mode = normalizeActivationMode(c.activationMode);
    const target = groups.find((g) => g.mode === mode);
    (target ?? groups[2]).items.push(c);
  }
  return groups.filter((g) => g.items.length > 0);
});
const selectedContact = computed(() =>
  currentChannelContacts.value.find((item) => item.id === selectedContactId.value) ?? null,
);
const contactLogsTarget = computed(() =>
  contacts.value.find((item) => item.id === contactLogsContactId.value) ?? null,
);
const contactModalTitle = computed(() => {
  if (!selectedContact.value) return "-";
  if (selectedContact.value.platform === "weixin_oc") return t("config.remoteIm.weixinContact");
  return contactDisplayName(selectedContact.value);
});
const contactLogsTitle = computed(() => {
  const target = contactLogsTarget.value;
  if (!target) return "-";
  return contactSafeDisplayName(target);
});
type ContactEditDraft = {
  boundDepartmentId: string;
  processingMode: "qa" | "continuous";
  activationMode: RemoteImContact["activationMode"];
  activationKeywordsText: string;
  muteKeywordsText: string;
  unmuteKeywordsText: string;
  responseStrategy: NonNullable<RemoteImContact["responseStrategy"]>;
  responseGuidance: string;
  patienceSeconds: number;
  muteDurationSeconds: number;
  allowReceive: boolean;
  allowSend: boolean;
  allowSendFiles: boolean;
  shellWorkspaces: ShellWorkspace[];
};
type ContactLogDisplayItem = {
  timestamp: string;
  level: string;
  kind: string;
  title: string;
  summary: string;
  detail?: string;
};
const contactDraft = ref<ContactEditDraft | null>(null);
const contactDraftSnapshot = ref("");
const contactDraftDirty = computed(() =>
  !!contactDraft.value && JSON.stringify(contactDraft.value) !== contactDraftSnapshot.value,
);
const contactDraftRoutingHint = computed(() => {
  if (!selectedContact.value || !contactDraft.value) return "";
  return t("config.remoteIm.routingHint");
});
const contactDraftProcessingHint = computed(() => {
  if (!selectedContact.value || !contactDraft.value) return "";
  return processingModeHintText({
    ...selectedContact.value,
    processingMode: contactDraft.value.processingMode,
  } as RemoteImContact);
});
const contactDraftActivationHint = computed(() => {
  if (!selectedContact.value || !contactDraft.value) return "";
  return contactActivationHintText({
    ...selectedContact.value,
    activationMode: contactDraft.value.activationMode,
  } as RemoteImContact);
});
const contactDraftResponseStrategyHint = computed(() => {
  if (!selectedContact.value || !contactDraft.value) return "";
  return contactResponseStrategyHintText({
    ...selectedContact.value,
    responseStrategy: contactDraft.value.responseStrategy,
  } as RemoteImContact);
});
const contactLogDisplayItems = computed<ContactLogDisplayItem[]>(() =>
  contactLogs.value
    .map((log) => buildContactLogDisplayItem(log))
    .filter((item): item is ContactLogDisplayItem => item !== null),
);
const contactLogDisplayLines = computed(() =>
  contactLogDisplayItems.value.map((item) => {
    const parts = [
      formatLogTime(item.timestamp),
      `[${item.kind}]`,
      item.title,
      item.summary,
      item.detail,
    ].filter((value) => String(value || "").trim().length > 0);
    return parts.join("  ");
  }),
);

const remoteImDepartmentOptions = computed(() =>
  (props.config.departments || [])
    .filter((dept) => dept.id !== "assistant-department" && !dept.isBuiltInAssistant)
    .map((dept) => ({
      id: dept.id,
      label: departmentDisplayName(dept),
    })),
);

const contactKeywordDrafts = ref<Record<string, string>>({});

function buildContactDraftFromContact(item: RemoteImContact): ContactEditDraft {
  return {
    boundDepartmentId: String(item.boundDepartmentId || ""),
    processingMode: normalizeProcessingMode(item.processingMode),
    activationMode: normalizeActivationMode(item.activationMode || "never"),
    activationKeywordsText: item.activationKeywords.join(", "),
    muteKeywordsText: (Array.isArray(item.muteKeywords) ? item.muteKeywords : [t("config.remoteIm.defaultMuteKeyword")]).join(", "),
    unmuteKeywordsText: (Array.isArray(item.unmuteKeywords) ? item.unmuteKeywords : [t("config.remoteIm.defaultUnmuteKeyword")]).join(", "),
    responseStrategy: normalizeResponseStrategy(item.responseStrategy),
    responseGuidance: String(item.responseGuidance || "").trim(),
    patienceSeconds: Math.max(0, Number(item.patienceSeconds || 60)),
    muteDurationSeconds: Math.max(0, Number(item.muteDurationSeconds || 600)),
    allowReceive: !!item.allowReceive,
    allowSend: !!item.allowSend,
    allowSendFiles: !!item.allowSendFiles,
    shellWorkspaces: (item as any).shellWorkspaces
      ? (item as any).shellWorkspaces.filter((ws: any) => ws.level !== "system").map((ws: any) => ({
          id: ws.id || crypto.randomUUID(),
          name: ws.name || "",
          path: ws.path || "",
          level: ws.level || "secondary",
          access: ws.access || "full_access",
        }))
      : [],
  };
}

function syncSelectedContactDraft() {
  if (!selectedContact.value) {
    contactDraft.value = null;
    contactDraftSnapshot.value = "";
    return;
  }
  const draft = buildContactDraftFromContact(selectedContact.value);
  contactDraft.value = draft;
  contactDraftSnapshot.value = JSON.stringify(draft);
}

function asNonEmptyString(value: unknown): string {
  return String(value || "").trim();
}

function validateChannelBeforeEnable(channel: RemoteImChannelConfig): string {
  const creds = channel.credentials || {};
  if (channel.platform === "dingtalk") {
    const clientId = asNonEmptyString(creds.clientId);
    const clientSecret = asNonEmptyString(creds.clientSecret);
    if (!clientId || !clientSecret) {
      return t("config.remoteIm.enableNeedDingtalkCredentials");
    }
  }
  if (channel.platform === "feishu") {
    const appId = asNonEmptyString(creds.appId);
    const appSecret = asNonEmptyString(creds.appSecret);
    if (!appId || !appSecret) {
      return t("config.remoteIm.enableNeedFeishuCredentials");
    }
  }
  if (channel.platform === "weixin_oc") {
    const baseUrl = asNonEmptyString(creds.baseUrl) || "https://ilinkai.weixin.qq.com";
    channel.credentials = {
      ...creds,
      baseUrl,
      botType: WEIXIN_OC_BOT_TYPE,
      qrPollInterval: WEIXIN_OC_QR_POLL_INTERVAL,
      longPollTimeoutMs: WEIXIN_OC_LONG_POLL_TIMEOUT_MS,
      apiTimeoutMs: WEIXIN_OC_API_TIMEOUT_MS,
    };
  }
  return "";
}

function defaultChannelName(platform: RemoteImPlatform): string {
  if (platform === "feishu") return "Feishu";
  if (platform === "dingtalk") return "DingTalk";
  if (platform === "weixin_oc") return "个人微信";
  return "OneBot v11";
}

function newChannel(platform: RemoteImPlatform = "onebot_v11"): RemoteImChannelConfig {
  return {
    id: `remote-im-${Date.now()}`,
    name: defaultChannelName(platform),
    platform,
    enabled: false,
    credentials: {},
    activateAssistant: true,
    receiveFiles: true,
    streamingSend: false,
    showToolCalls: false,
    allowSendFiles: false,
  };
}

function openAddChannelModal() {
  addChannelModalOpen.value = true;
}

function closeAddChannelModal() {
  addChannelModalOpen.value = false;
}

function addChannel(platform: RemoteImPlatform) {
  const ch = newChannel(platform);
  props.config.remoteImChannels.push(ch);
  selectedChannelId.value = ch.id;
  channelConfigModalOpen.value = true;
  addChannelModalOpen.value = false;
}

function removeChannelById(channelId: string) {
  const idx = channels.value.findIndex((ch) => ch.id === channelId);
  if (idx >= 0) {
    props.config.remoteImChannels.splice(idx, 1);
    if (selectedChannelId.value === channelId) {
      const nextIdx = Math.min(idx, channels.value.length - 1);
      selectedChannelId.value = nextIdx >= 0 ? channels.value[nextIdx].id : "";
    }
  }
}

function syncCredentialJson(channel: RemoteImChannelConfig) {
  const raw = String(credentialDrafts.value[channel.id] || "").trim();
  if (!raw) {
    channel.credentials = {};
    return;
  }
  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("credentials json must be object");
    }
    channel.credentials = parsed as Record<string, unknown>;
  } catch (error) {
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  }
}

function loadNapcatCredentials(channel: RemoteImChannelConfig) {
  suppressCredentialSync.value = true;
  const creds = channel.credentials || {};
  napcatCredentials.value = {
    wsHost: String(creds.wsHost || "0.0.0.0"),
    wsPort: Number(creds.wsPort) || 6199,
    wsToken: String(creds.wsToken || ""),
  };
  nextTick(() => {
    suppressCredentialSync.value = false;
  });
}

function loadDingtalkCredentials(channel: RemoteImChannelConfig) {
  suppressCredentialSync.value = true;
  const creds = channel.credentials || {};
  dingtalkCredentials.value = {
    clientId: String(creds.clientId || creds.clientID || ""),
    clientSecret: String(creds.clientSecret || creds.appSecret || ""),
  };
  nextTick(() => {
    suppressCredentialSync.value = false;
  });
}

function loadWeixinCredentials(channel: RemoteImChannelConfig) {
  suppressCredentialSync.value = true;
  const creds = channel.credentials || {};
  weixinCredentials.value = {
    baseUrl: String(creds.baseUrl || "https://ilinkai.weixin.qq.com"),
    botType: WEIXIN_OC_BOT_TYPE,
    qrPollInterval: WEIXIN_OC_QR_POLL_INTERVAL,
    longPollTimeoutMs: WEIXIN_OC_LONG_POLL_TIMEOUT_MS,
    apiTimeoutMs: WEIXIN_OC_API_TIMEOUT_MS,
  };
  nextTick(() => {
    suppressCredentialSync.value = false;
  });
}

function resetNapcatCredentials() {
  if (!selectedChannel.value) return;
  loadNapcatCredentials(selectedChannel.value);
  // 同时更新 channelSnapshot 以清除 dirty 状态
  lastSavedChannelSnapshot.value = channelSnapshot.value;
}

async function saveChannels() {
  if (saving.value || !selectedChannel.value) return false;
  if (selectedChannel.value.platform === "feishu") {
    syncCredentialJson(selectedChannel.value);
  }
  const savedId = selectedChannelId.value;
  saving.value = true;
  try {
    const result = await Promise.resolve(props.saveConfigAction());
    if (result) {
      if (channels.value.some((ch) => ch.id === savedId)) {
        selectedChannelId.value = savedId;
      }
      if (selectedChannel.value) {
        if (selectedChannel.value.platform === "onebot_v11") {
          loadNapcatCredentials(selectedChannel.value);
        } else if (selectedChannel.value.platform === "dingtalk") {
          loadDingtalkCredentials(selectedChannel.value);
        } else if (selectedChannel.value.platform === "weixin_oc") {
          loadWeixinCredentials(selectedChannel.value);
        }
        if (selectedChannel.value.platform === "onebot_v11" || selectedChannel.value.platform === "dingtalk" || selectedChannel.value.platform === "weixin_oc") {
          try {
            const status = await invokeTauri<ChannelConnectionStatus>(
              "remote_im_restart_channel",
              { channelId: selectedChannel.value.id },
            );
            channelStatus.value = status;
            channelRuntimeStates.value = {
              ...channelRuntimeStates.value,
              [selectedChannel.value.id]: status,
            };
          } catch (err) {
            console.warn("[RemoteImTab] restart channel failed:", err);
            void refreshChannelStatus();
          }
        }
      }
      await nextTick();
      lastSavedChannelSnapshot.value = channelSnapshot.value;
      return true;
    }
    return false;
  } finally {
    saving.value = false;
  }
}

async function toggleChannelEnabled(channel: RemoteImChannelConfig, enabled: boolean) {
  const previousEnabled = channel.enabled;
  props.setStatusAction(`正在${enabled ? "启用" : "停用"}渠道：${channel.name || channel.id}`);
  if (enabled) {
    const validationError = validateChannelBeforeEnable(channel);
    if (validationError) {
      props.setStatusAction(validationError);
      return;
    }
  }
  channel.enabled = enabled;
  saving.value = true;
  try {
    const result = await Promise.resolve(props.saveConfigAction());
    if (result) {
      props.setStatusAction(enabled ? "渠道已启用" : "渠道已停用");
      if (channel.platform === "onebot_v11" || channel.platform === "dingtalk" || channel.platform === "weixin_oc") {
        try {
          const status = await invokeTauri<ChannelConnectionStatus>(
            "remote_im_restart_channel",
            { channelId: channel.id },
          );
          channelStatus.value = status;
          channelRuntimeStates.value = {
            ...channelRuntimeStates.value,
            [channel.id]: status,
          };
        } catch (err) {
          console.warn("[RemoteImTab] restart channel failed:", err);
          props.setStatusAction(`渠道重启失败，开关未完全生效: ${String(err)}`);
          void refreshChannelStatus();
        }
      }
      await nextTick();
      lastSavedChannelSnapshot.value = channelSnapshot.value;
    } else {
      channel.enabled = previousEnabled;
      props.setStatusAction("保存失败，渠道状态未生效。");
    }
  } catch (error) {
    channel.enabled = previousEnabled;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  } finally {
    saving.value = false;
  }
}

async function toggleSelectedChannelEnabled(enabled: boolean) {
  if (!selectedChannel.value) return;
  await toggleChannelEnabled(selectedChannel.value, enabled);
}

async function toggleContactCommunication(item: RemoteImContact, enabled: boolean) {
  if (contactsDisabled.value) return;
  const oldSend = item.allowSend;
  const oldReceive = item.allowReceive;
  item.allowSend = enabled;
  item.allowReceive = enabled;
  try {
    await invokeTauri<RemoteImContact>("remote_im_update_contact_allow_send", {
      input: { contactId: item.id, allowSend: enabled },
    });
    await invokeTauri<RemoteImContact>("remote_im_update_contact_allow_receive", {
      input: { contactId: item.id, allowReceive: enabled },
    });
    await refreshContacts();
  } catch (error) {
    item.allowSend = oldSend;
    item.allowReceive = oldReceive;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
    await refreshContacts();
  }
}

async function toggleContactAllowSendFiles(item: RemoteImContact, enabled: boolean) {
  if (contactsDisabled.value) return;
  const oldValue = item.allowSendFiles;
  item.allowSendFiles = enabled;
  try {
    await invokeTauri<RemoteImContact>("remote_im_update_contact_allow_send_files", {
      input: { contactId: item.id, allowSendFiles: enabled },
    });
    await refreshContacts();
  } catch (error) {
    item.allowSendFiles = oldValue;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  }
}

async function saveContactActivation(
  item: RemoteImContact,
  patch?: Partial<
    Pick<
      RemoteImContact,
      | "activationMode"
      | "activationKeywords"
      | "muteKeywords"
      | "unmuteKeywords"
      | "patienceSeconds"
      | "muteDurationSeconds"
      | "activationCooldownSeconds"
      | "responseStrategy"
      | "responseGuidance"
    >
  >,
) {
  if (contactsDisabled.value) return;
  const oldMode = item.activationMode;
  const oldKeywords = [...item.activationKeywords];
  const oldMuteKeywords = [...(Array.isArray(item.muteKeywords) ? item.muteKeywords : [])];
  const oldUnmuteKeywords = [...(Array.isArray(item.unmuteKeywords) ? item.unmuteKeywords : [])];
  const oldPatience = item.patienceSeconds;
  const oldMuteDuration = item.muteDurationSeconds;
  const oldCooldown = item.activationCooldownSeconds;
  const oldResponseStrategy = normalizeResponseStrategy(item.responseStrategy);
  const oldResponseGuidance = String(item.responseGuidance || "");
  if (patch?.activationMode) item.activationMode = patch.activationMode;
  if (patch?.activationKeywords) item.activationKeywords = [...patch.activationKeywords];
  if (patch?.muteKeywords) item.muteKeywords = [...patch.muteKeywords];
  if (patch?.unmuteKeywords) item.unmuteKeywords = [...patch.unmuteKeywords];
  if (typeof patch?.patienceSeconds === "number") {
    item.patienceSeconds = Math.max(0, Math.floor(patch.patienceSeconds));
  }
  if (typeof patch?.muteDurationSeconds === "number") {
    item.muteDurationSeconds = Math.max(0, Math.floor(patch.muteDurationSeconds));
  }
  if (typeof patch?.activationCooldownSeconds === "number") {
    item.activationCooldownSeconds = Math.max(0, Math.floor(patch.activationCooldownSeconds));
  }
  if (patch?.responseStrategy) item.responseStrategy = normalizeResponseStrategy(patch.responseStrategy);
  if (typeof patch?.responseGuidance === "string") item.responseGuidance = patch.responseGuidance;
  try {
    await invokeTauri<RemoteImContact>("remote_im_update_contact_activation", {
      input: {
        contactId: item.id,
        activationMode: item.activationMode,
        activationKeywords: item.activationKeywords,
        muteKeywords: item.muteKeywords,
        unmuteKeywords: item.unmuteKeywords,
        patienceSeconds: item.patienceSeconds,
        muteDurationSeconds: item.muteDurationSeconds,
        activationCooldownSeconds: item.activationCooldownSeconds,
        responseStrategy: normalizeResponseStrategy(item.responseStrategy),
        responseGuidance: String(item.responseGuidance || ""),
      },
    });
    await refreshContacts();
  } catch (error) {
    item.activationMode = oldMode;
    item.activationKeywords = oldKeywords;
    item.muteKeywords = oldMuteKeywords;
    item.unmuteKeywords = oldUnmuteKeywords;
    item.patienceSeconds = oldPatience;
    item.muteDurationSeconds = oldMuteDuration;
    item.activationCooldownSeconds = oldCooldown;
    item.responseStrategy = oldResponseStrategy;
    item.responseGuidance = oldResponseGuidance;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  }
}

function onContactActivationModeChange(item: RemoteImContact, modeRaw: string) {
  const mode = normalizeActivationMode(modeRaw);
  void saveContactActivation(item, { activationMode: mode });
}

async function onContactDepartmentChange(
  item: RemoteImContact,
  departmentIdRaw: string,
) {
  const oldValue = item.boundDepartmentId;
  const nextDepartmentId = String(departmentIdRaw || "").trim() || "";
  item.boundDepartmentId = nextDepartmentId || undefined;
  try {
    await invokeTauri<RemoteImContact>("remote_im_update_contact_department_binding", {
      input: {
        contactId: item.id,
        departmentId: nextDepartmentId || null,
      },
    });
    props.setStatusAction("联系人消息将继续使用该联系人的独立会话。");
    await refreshContacts();
  } catch (error) {
    item.boundDepartmentId = oldValue;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  }
}

async function onContactProcessingModeChange(
  item: RemoteImContact,
  processingModeRaw: string,
) {
  const oldValue = normalizeProcessingMode(item.processingMode);
  item.processingMode = normalizeProcessingMode(processingModeRaw);
  try {
    await invokeTauri<RemoteImContact>("remote_im_update_contact_processing_mode", {
      input: {
        contactId: item.id,
        processingMode: item.processingMode,
      },
    });
    await refreshContacts();
  } catch (error) {
    item.processingMode = oldValue;
    props.setStatusAction(t("status.saveConfigFailed", { err: String(error) }));
  }
}

function onContactActivationKeywordsBlur(item: RemoteImContact) {
  const raw = contactKeywordDrafts.value[item.id] ?? item.activationKeywords.join(", ");
  const keywords = parseActivationKeywords(raw);
  contactKeywordDrafts.value[item.id] = keywords.join(", ");
  void saveContactActivation(item, { activationKeywords: keywords });
}

async function addContactWorkspace() {
  if (!contactDraft.value) return;
  try {
    const picked = await open({
      directory: true,
      multiple: false,
    });
    if (!picked || Array.isArray(picked)) return;
    const path = String(picked || "").trim();
    if (!path) return;

    const existed = contactDraft.value.shellWorkspaces.some(
      (ws) => ws.path.toLowerCase() === path.toLowerCase(),
    );
    if (existed) return;

    const hasMain = contactDraft.value.shellWorkspaces.some((ws) => ws.level === "main");
    const name = path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() || path;
    contactDraft.value.shellWorkspaces.push({
      id: crypto.randomUUID(),
      name,
      path,
      level: hasMain ? "secondary" : "main",
      access: hasMain ? "read_only" : "approval",
    });
  } catch {
    // 用户取消选择
  }
}

function removeContactWorkspace(id: string) {
  if (!contactDraft.value) return;
  const idx = contactDraft.value.shellWorkspaces.findIndex((ws) => ws.id === id);
  if (idx >= 0) contactDraft.value.shellWorkspaces.splice(idx, 1);
}

function setContactWorkspaceMain(id: string) {
  if (!contactDraft.value) return;
  for (const ws of contactDraft.value.shellWorkspaces) {
    ws.level = ws.id === id ? "main" : "secondary";
  }
}

function updateContactWorkspaceAccess(id: string, access: ShellWorkspace["access"]) {
  if (!contactDraft.value) return;
  const ws = contactDraft.value.shellWorkspaces.find((w) => w.id === id);
  if (ws) ws.access = access;
}

function resetContactDraft() {
  syncSelectedContactDraft();
}

async function saveContactDraft() {
  if (contactsDisabled.value) return;
  if (!selectedContact.value || !contactDraft.value || !contactDraftDirty.value || contactSaving.value) return;
  const item = selectedContact.value;
  const draft = contactDraft.value;
  contactSaving.value = true;
  try {
    const nextDepartmentId = String(draft.boundDepartmentId || "").trim();
    const currentDepartmentId = String(item.boundDepartmentId || "").trim();
    if (nextDepartmentId !== currentDepartmentId) {
      await onContactDepartmentChange(item, nextDepartmentId);
    }

    const nextProcessingMode = normalizeProcessingMode(draft.processingMode);
    if (nextProcessingMode !== normalizeProcessingMode(item.processingMode)) {
      await onContactProcessingModeChange(item, nextProcessingMode);
    }

    const nextKeywords = parseActivationKeywords(draft.activationKeywordsText);
    const nextMuteKeywords = parseKeywordList(draft.muteKeywordsText);
    const nextUnmuteKeywords = parseKeywordList(draft.unmuteKeywordsText);
    const currentKeywords = Array.isArray(item.activationKeywords) ? item.activationKeywords : [];
    const currentMuteKeywords = Array.isArray(item.muteKeywords) ? item.muteKeywords : [t("config.remoteIm.defaultMuteKeyword")];
    const currentUnmuteKeywords = Array.isArray(item.unmuteKeywords) ? item.unmuteKeywords : [t("config.remoteIm.defaultUnmuteKeyword")];
    const keywordsChanged = JSON.stringify(nextKeywords) !== JSON.stringify(currentKeywords);
    const muteKeywordsChanged =
      JSON.stringify(nextMuteKeywords) !== JSON.stringify(currentMuteKeywords);
    const unmuteKeywordsChanged =
      JSON.stringify(nextUnmuteKeywords) !== JSON.stringify(currentUnmuteKeywords);
    const nextActivationMode = normalizeActivationMode(draft.activationMode);
    const modeChanged = nextActivationMode !== normalizeActivationMode(item.activationMode || "never");
    const nextResponseStrategy = normalizeResponseStrategy(draft.responseStrategy);
    const responseStrategyChanged =
      nextResponseStrategy !== normalizeResponseStrategy(item.responseStrategy);
    const nextResponseGuidance = String(draft.responseGuidance || "").trim();
    const currentResponseGuidance = String(item.responseGuidance || "").trim();
    const responseGuidanceChanged = nextResponseGuidance !== currentResponseGuidance;
    const nextPatience = Math.max(0, Math.floor(Number(draft.patienceSeconds) || 0));
    const patienceChanged = nextPatience !== Math.max(0, Math.floor(Number(item.patienceSeconds || 60)));
    const nextMuteDuration = Math.max(0, Math.floor(Number(draft.muteDurationSeconds) || 0));
    const muteDurationChanged =
      nextMuteDuration !== Math.max(0, Math.floor(Number(item.muteDurationSeconds || 600)));
    if (
      modeChanged
      || keywordsChanged
      || muteKeywordsChanged
      || unmuteKeywordsChanged
      || patienceChanged
      || muteDurationChanged
      || responseStrategyChanged
      || responseGuidanceChanged
    ) {
      await saveContactActivation(item, {
        activationMode: nextActivationMode,
        activationKeywords: nextKeywords,
        muteKeywords: nextMuteKeywords,
        unmuteKeywords: nextUnmuteKeywords,
        patienceSeconds: nextPatience,
        muteDurationSeconds: nextMuteDuration,
        responseStrategy: nextResponseStrategy,
        responseGuidance: nextResponseGuidance,
      });
    }

    if (!!draft.allowReceive !== !!item.allowReceive || !!draft.allowSend !== !!item.allowSend) {
      await toggleContactCommunication(item, !!draft.allowReceive || !!draft.allowSend);
    }
    if (!!draft.allowSendFiles !== !!item.allowSendFiles) {
      await toggleContactAllowSendFiles(item, !!draft.allowSendFiles);
    }
    // 保存联系人工作区配置
    try {
      await invokeTauri<RemoteImContact>("remote_im_update_contact_workspace", {
        input: {
          contactId: item.id,
          shellWorkspaces: (draft.shellWorkspaces || []).map((ws) => ({
            id: ws.id,
            name: ws.name,
            path: ws.path,
            level: ws.level,
            access: ws.access,
            builtIn: false,
          })),
        },
      });
    } catch (e) {
      console.error("[联系人工作区保存失败]", e);
      props.setStatusAction(t("status.saveConfigFailed", { err: String(e) }));
      return;
    }
    await refreshContacts();
    syncSelectedContactDraft();
  } finally {
    contactSaving.value = false;
  }
}

async function startWeixinLogin() {
  if (!selectedChannel.value || selectedChannel.value.platform !== "weixin_oc") return;
  weixinLoginBusy.value = true;
  try {
    const result = await invokeTauri<WeixinLoginStatus | {
      channelId: string;
      sessionKey: string;
      qrcode: string;
      qrcodeImgContent: string;
      status: string;
      message: string;
    }>("remote_im_weixin_oc_start_login", {
      input: {
        channelId: selectedChannel.value.id,
        forceRefresh: true,
      },
    });
    weixinLoginStates.value = {
      ...weixinLoginStates.value,
      [selectedChannel.value.id]: {
        channelId: result.channelId,
        connected: false,
        status: result.status,
        message: result.message,
        sessionKey: result.sessionKey,
        qrcode: result.qrcode,
        qrcodeImgContent: result.qrcodeImgContent,
        accountId: "",
        userId: "",
        baseUrl: "",
        lastError: "",
      },
    };
    if (weixinLoginPollTimer) clearInterval(weixinLoginPollTimer);
    weixinLoginPollTimer = setInterval(() => {
      void pollWeixinLoginStatus();
    }, 2500);
  } catch (error) {
    props.setStatusAction(`个人微信扫码登录失败: ${String(error)}`);
  } finally {
    weixinLoginBusy.value = false;
  }
}

async function onWeixinLoginButtonClick() {
  if (weixinLoginBusy.value) return;
  if (channelDirty.value) {
    props.setStatusAction("正在保存微信渠道配置...");
    const saved = await saveChannels();
    if (!saved) {
      props.setStatusAction("请先保存当前微信渠道配置，再进行扫码登录。");
      return;
    }
  }
  if (isWeixinLoggedIn.value) {
    await logoutWeixin();
  }
  await startWeixinLogin();
}

async function pollWeixinLoginStatus() {
  if (!selectedChannel.value || selectedChannel.value.platform !== "weixin_oc") return;
  const channelId = selectedChannel.value.id;
  try {
    const result = await invokeTauri<WeixinLoginStatus>("remote_im_weixin_oc_get_login_status", {
      input: { channelId },
    });
    weixinLoginStates.value = {
      ...weixinLoginStates.value,
      [channelId]: result,
    };
    if (result.connected || result.status === "expired") {
      if (weixinLoginPollTimer) {
        clearInterval(weixinLoginPollTimer);
        weixinLoginPollTimer = null;
      }
      if (result.connected) {
        await refreshChannelStatus();
        await refreshContacts();
      }
    }
  } catch (error) {
    const errMsg = `个人微信登录状态查询失败: ${String(error)}`;
    weixinLoginStates.value = {
      ...weixinLoginStates.value,
      [channelId]: {
        ...(weixinLoginStates.value[channelId] || {
          channelId,
          connected: false,
          status: "wait",
          message: "",
          sessionKey: "",
          qrcode: "",
          qrcodeImgContent: "",
          accountId: "",
          userId: "",
          baseUrl: "",
          lastError: "",
        }),
        message: errMsg,
        lastError: errMsg,
      },
    };
    props.setStatusAction(errMsg);
  }
}

async function syncWeixinContacts() {
  if (!selectedChannel.value || selectedChannel.value.platform !== "weixin_oc") return;
  try {
    const result = await invokeTauri<{ message: string }>("remote_im_weixin_oc_sync_contacts", {
      input: { channelId: selectedChannel.value.id },
    });
    props.setStatusAction(result.message);
    await refreshContacts();
  } catch (error) {
    props.setStatusAction(`个人微信联系人同步失败: ${String(error)}`);
  }
}

async function logoutWeixin() {
  if (!selectedChannel.value || selectedChannel.value.platform !== "weixin_oc") return;
  try {
    await invokeTauri<boolean>("remote_im_weixin_oc_logout", {
      input: { channelId: selectedChannel.value.id },
    });
    weixinLoginStates.value = {
      ...weixinLoginStates.value,
      [selectedChannel.value.id]: {
        channelId: selectedChannel.value.id,
        connected: false,
        status: "logged_out",
        message: "已退出登录",
      },
    };
    await refreshChannelStatus();
    props.setStatusAction("个人微信已退出登录。");
  } catch (error) {
    props.setStatusAction(`个人微信退出登录失败: ${String(error)}`);
  }
}

async function refreshContacts() {
  contactsLoading.value = true;
  contactsError.value = "";
  try {
    contacts.value = await invokeTauri<RemoteImContact[]>("remote_im_list_contacts");
    for (const item of contacts.value) {
      item.activationMode = normalizeActivationMode(item.activationMode || "never");
      item.activationKeywords = Array.isArray(item.activationKeywords) ? item.activationKeywords : [];
      item.muteKeywords = Array.isArray(item.muteKeywords) && item.muteKeywords.length > 0 ? item.muteKeywords : [t("config.remoteIm.defaultMuteKeyword")];
      item.unmuteKeywords =
        Array.isArray(item.unmuteKeywords) && item.unmuteKeywords.length > 0 ? item.unmuteKeywords : [t("config.remoteIm.defaultUnmuteKeyword")];
      item.activationCooldownSeconds = Math.max(0, Number(item.activationCooldownSeconds || 0));
      item.processingMode = normalizeProcessingMode(item.processingMode);
      item.responseStrategy = normalizeResponseStrategy(item.responseStrategy);
      item.responseGuidance = String(item.responseGuidance || "").trim();
      item.muteDurationSeconds = Math.max(0, Number(item.muteDurationSeconds || 600));
      item.allowSendFiles = !!item.allowSendFiles;
      contactKeywordDrafts.value[item.id] = item.activationKeywords.join(", ");
    }
    if (selectedContactId.value && !contacts.value.some((item) => item.id === selectedContactId.value)) {
      contactConfigModalOpen.value = false;
      selectedContactId.value = "";
    }
    if (contactLogsContactId.value && !contacts.value.some((item) => item.id === contactLogsContactId.value)) {
      contactLogsModalOpen.value = false;
      contactLogsContactId.value = "";
      contactLogs.value = [];
    }
  } catch (error) {
    contactsError.value = String(error);
  } finally {
    contactsLoading.value = false;
  }
}

function contactDisplayName(item: RemoteImContact): string {
  const remark = String(item.remarkName || "").trim();
  if (remark) return remark;
  const remoteName = String(item.remoteContactName || "").trim();
  if (remoteName) return remoteName;
  return item.remoteContactId;
}

function contactSafeDisplayName(item: RemoteImContact): string {
  if (item.platform === "weixin_oc") {
    const remark = String(item.remarkName || "").trim();
    if (remark) return remark;
    const remoteName = String(item.remoteContactName || "").trim();
    if (remoteName && !remoteName.includes("@")) return remoteName;
    return "微信联系人";
  }
  return contactDisplayName(item);
}

function contactSecondaryText(item: RemoteImContact): string {
  if (item.platform === "weixin_oc") {
    return item.remoteContactType === "group" ? "微信群联系人" : "微信个人联系人";
  }
  return item.remoteContactId;
}

function contactLogField(message: string, key: string): string {
  const match = message.match(new RegExp(`${key}=([\\s\\S]*?)(?=, [a-z_]+=|$)`));
  return String(match?.[1] || "").trim();
}

function contactLogTransitionLabel(value: string): string {
  const normalized = String(value || "").trim();
  if (!normalized) return "";
  const [fromRaw, toRaw] = normalized.split("->").map((item) => item.trim());
  if (!toRaw) return normalized;
  if (!fromRaw || fromRaw === toRaw) return toRaw;
  return `${fromRaw} -> ${toRaw}`;
}

function contactLogCurrentStateLabel(value: string): string {
  const normalized = String(value || "").trim();
  if (!normalized) return "";
  const parts = normalized.split("->").map((item) => item.trim()).filter(Boolean);
  return parts[parts.length - 1] || "";
}

function contactLogHumanName(value: string): string {
  return String(value || "")
    .trim()
    .replace(/\(\d+\)\s*$/, "")
    .trim();
}

function contactLogHumanId(value: string): string {
  const match = String(value || "").trim().match(/\((\d+)\)\s*$/);
  return String(match?.[1] || "").trim();
}

function contactLogBoolLabel(value: string): string {
  return value === "是" || value.toLowerCase() === "true" ? t("common.yes") : t("common.no");
}

function contactLogModeLabel(value: string): string {
  if (value === "direct") return t("config.remoteIm.logModeDirect");
  if (value === "queued") return t("config.remoteIm.logModeQueued");
  if (value === "duplicate") return t("config.remoteIm.logModeDuplicate");
  return value || "-";
}

function contactLogDecisionLabel(value: string): string {
  if (value === "reply" || value === "reply_async") return t("config.remoteIm.logDecisionReply");
  if (value === "send") return t("config.remoteIm.logDecisionSend");
  if (value === "send_files") return t("config.remoteIm.logDecisionSendFiles");
  if (value === "no_reply") return t("config.remoteIm.logDecisionNoReply");
  if (value === "send_async") return t("config.remoteIm.logDecisionSendAsync");
  return value || t("common.done");
}

function contactLogStateSummary(message: string): string {
  const presence = contactLogCurrentStateLabel(contactLogField(message, "presence"));
  const work = contactLogCurrentStateLabel(contactLogField(message, "work"));
  const activate = contactLogField(message, "activate");
  const parts = [
    presence,
    work,
    activate ? (contactLogBoolLabel(activate) === t("common.yes") ? t("config.remoteIm.logActivate") : t("config.remoteIm.logInactive")) : "",
  ].filter(Boolean);
  return parts.join("；");
}

function buildContactLogDisplayItem(log: ChannelLogEntry): ContactLogDisplayItem | null {
  const message = String(log.message || "").trim();
  if (message.startsWith("[联系人消息] 收到:")) {
    const senderRaw = contactLogField(message, "sender");
    const senderName = contactLogHumanName(senderRaw) || "对方";
    const senderId = contactLogHumanId(senderRaw);
    const preview = contactLogField(message, "preview") || "收到一条消息";
    const imageCount = Number(contactLogField(message, "image_count") || 0);
    const audioCount = Number(contactLogField(message, "audio_count") || 0);
    const attachmentCount = Number(contactLogField(message, "attachment_count") || 0);
    const extras = [
      imageCount > 0 ? `图片 ${imageCount}` : "",
      audioCount > 0 ? `音频 ${audioCount}` : "",
      attachmentCount > 0 ? `附件 ${attachmentCount}` : "",
    ].filter(Boolean);
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "消息",
      title: "",
      summary: `${senderId ? `[${senderName}/${senderId}]` : `[${senderName}]`}${preview}`,
      detail: extras.length > 0 ? extras.join("，") : undefined,
    };
  }
  if (message.startsWith("[联系人消息] 去重跳过:")) {
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "去重",
      title: "重复消息已跳过",
      summary: contactLogField(message, "preview") || "这条消息之前已经处理过",
    };
  }
  if (message.startsWith("[联系人消息] 入队:")) {
    return log.level === "warn" || log.level === "error"
      ? {
          timestamp: log.timestamp,
          level: log.level,
          kind: "系统",
          title: "消息入队失败",
          summary: contactLogField(message, "reason") || "消息没有成功进入处理链路",
        }
      : null;
  }
  if (message.startsWith("[联系人状态] 入站判定:")) {
    const reason = contactLogField(message, "reason");
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "状态",
      title: contactLogStateSummary(message),
      summary: reason ? `原因：${reason}` : "",
    };
  }
  if (message.startsWith("[联系人状态] 历史落地:")) {
    return log.level === "warn" || log.level === "error"
      ? {
          timestamp: log.timestamp,
          level: log.level,
          kind: "系统",
          title: "消息写入历史失败",
          summary: contactLogField(message, "reason") || "历史落地失败",
        }
      : null;
  }
  if (message.startsWith("[联系人消息] 发出失败:")) {
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "发送",
      title: "[发送失败]",
      summary: contactLogField(message, "preview") || "发送内容已省略",
      detail: contactLogField(message, "error") || undefined,
    };
  }
  if (message.startsWith("[联系人消息] 发出跳过:")) {
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "发送",
      title: "[跳过发送]",
      summary: contactLogField(message, "reason") || "本次没有对外发送",
    };
  }
  if (message.startsWith("[联系人消息] 发出:")) {
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "发送",
      title: "",
      summary: `[发送消息]${contactLogField(message, "preview") || "发送内容已省略"}`,
    };
  }
  if (message.startsWith("[联系人状态] 轮次结束:")) {
    const decision = contactLogDecisionLabel(contactLogField(message, "decision"));
    const followUp = contactLogBoolLabel(contactLogField(message, "follow_up"));
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "状态",
      title: contactLogStateSummary(message),
      summary: contactLogStateSummary(message),
      detail: `本轮处理：${decision}；继续跟进：${followUp}`,
    };
  }
  if (message.startsWith("[联系人状态] 轮次收尾失败:")) {
    return {
      timestamp: log.timestamp,
      level: log.level,
      kind: "状态",
      title: contactLogStateSummary(message),
      summary: contactLogStateSummary(message),
      detail: contactLogField(message, "error") || undefined,
    };
  }
  if (message.startsWith("[联系人状态] 异步发送收尾:")) {
    return null;
  }
  return log.level === "warn" || log.level === "error"
    ? {
        timestamp: log.timestamp,
        level: log.level,
        kind: "系统",
        title: "系统记录了一条异常日志",
        summary: "这条日志还没有整理成人话格式，但因为它是异常，所以保留显示。",
      }
    : null;
}

async function deleteContact(item: RemoteImContact) {
  if (contactsDisabled.value) return;
  if (contactDeleting.value) return;
  const displayName = contactSafeDisplayName(item);
  const confirmed = window.confirm(`确定删除联系人“${displayName}”吗？\n仅删除联系人记录，不清理已有会话历史。`);
  if (!confirmed) return;
  contactDeleting.value = true;
  try {
    const removed = await invokeTauri<boolean>("remote_im_delete_contact", {
      input: { contactId: item.id },
    });
    if (!removed) {
      props.setStatusAction(`删除联系人失败：未找到 ${displayName}`);
      return;
    }
    if (selectedContactId.value === item.id) {
      contactConfigModalOpen.value = false;
      selectedContactId.value = "";
      contactDraft.value = null;
      contactDraftSnapshot.value = "";
    }
    await refreshContacts();
    props.setStatusAction(`已删除联系人：${displayName}。已有会话历史已保留。`);
  } catch (error) {
    props.setStatusAction(`删除联系人失败：${String(error)}`);
  } finally {
    contactDeleting.value = false;
  }
}

function contactDepartmentLabel(item: RemoteImContact): string {
  const departmentId = String(item.boundDepartmentId || "").trim();
  if (!departmentId) return t("config.department.assistantBadge");
  const matched = remoteImDepartmentOptions.value.find((dept) => dept.id === departmentId);
  return matched ? matched.label : departmentId;
}

function departmentDisplayName(dept: DepartmentConfig): string {
  const id = String(dept.id || "").trim();
  if (id === "remote-customer-service-department") {
    return t("config.department.defaults.remoteCustomerServiceName");
  }
  return String(dept.name || "").trim() || id;
}

function contactProcessingModeLabel(item: RemoteImContact): string {
  return normalizeProcessingMode(item.processingMode) === "qa"
    ? t("config.remoteIm.processingModeQa")
    : t("config.remoteIm.processingModeContinuous");
}

function processingModeHintText(item: RemoteImContact): string {
  return normalizeProcessingMode(item.processingMode) === "qa"
    ? t("config.remoteIm.processingModeQaHint")
    : t("config.remoteIm.processingModeContinuousHint");
}

function contactActivationModeLabel(item: RemoteImContact): string {
  const mode = normalizeActivationMode(item.activationMode || "never");
  if (mode === "always") return t("config.remoteIm.activateModeAlways");
  if (mode === "keyword") return t("config.remoteIm.activateModeKeyword");
  return t("config.remoteIm.activateModeNever");
}

function contactActivationHintText(item: RemoteImContact): string {
  const mode = normalizeActivationMode(item.activationMode);
  if (mode === "always") return t("config.remoteIm.activateModeAlwaysHint");
  if (mode === "keyword") return t("config.remoteIm.activateModeKeywordHint");
  return t("config.remoteIm.activateModeNeverHint");
}

function contactResponseStrategyHintText(item: RemoteImContact): string {
  return normalizeResponseStrategy(item.responseStrategy) === "smart_judge"
    ? t("config.remoteIm.responseStrategySmartHint")
    : t("config.remoteIm.responseStrategyAlwaysHint");
}

function contactNeedsQuickModel(item: RemoteImContact): boolean {
  return normalizeResponseStrategy(item.responseStrategy) === "smart_judge";
}

function platformLabelText(platform: string): string {
  const value = String(platform || "").trim().toLowerCase();
  if (value === "weixin_oc") return t("config.remoteIm.platformOptions.weixinOc");
  if (value === "feishu") return t("config.remoteIm.platformOptions.feishu");
  if (value === "dingtalk") return t("config.remoteIm.platformOptions.dingtalk");
  return t("config.remoteIm.platformOptions.onebotV11");
}

async function refreshChannelStatus() {
  if (!selectedChannel.value) return;
  const channelId = selectedChannel.value.id;
  try {
    const status = await invokeTauri<ChannelConnectionStatus>("remote_im_get_channel_status", {
      channelId,
    });
    if (selectedChannel.value?.id === channelId) {
      channelStatus.value = status;
    }
    channelRuntimeStates.value = {
      ...channelRuntimeStates.value,
      [channelId]: status,
    };
  } catch (error) {
    console.error("[RemoteImTab] refreshChannelStatus failed:", error);
    if (selectedChannel.value?.id === channelId) {
      channelStatus.value = null;
    }
    channelRuntimeStates.value = {
      ...channelRuntimeStates.value,
      [channelId]: null,
    };
  }
}

async function refreshChannelStatusById(channelId: string) {
  try {
    const status = await invokeTauri<ChannelConnectionStatus>("remote_im_get_channel_status", { channelId });
    channelRuntimeStates.value = {
      ...channelRuntimeStates.value,
      [channelId]: status,
    };
    if (selectedChannel.value?.id === channelId) {
      channelStatus.value = status;
    }
  } catch {
    channelRuntimeStates.value = {
      ...channelRuntimeStates.value,
      [channelId]: null,
    };
    if (selectedChannel.value?.id === channelId) {
      channelStatus.value = null;
    }
  }
}

async function refreshAllChannelStatuses() {
  const jobs = channels.value
    .filter((item) => item.platform === "onebot_v11" || item.platform === "dingtalk" || item.platform === "weixin_oc")
    .map((item) => refreshChannelStatusById(item.id));
  await Promise.all(jobs);
}

function channelStatusPreview(channel: RemoteImChannelConfig): string {
  if (channel.platform === "weixin_oc") {
    const status = channelRuntimeStates.value[channel.id];
    if (!status) return "未初始化";
    if (status.connected) return "已连接";
    if (!channel.enabled) {
      if (status.statusText === "confirmed" || status.statusText === "logged_in") {
        return "已登录未启用";
      }
      if (status.accountId) return "已登录未启用";
      if (status.statusText === "need_login") return "未启用（待扫码登录）";
      return t("config.remoteIm.disabledState");
    }
    if (status.statusText === "need_login") return "待扫码登录";
    if (status.statusText === "confirmed" || status.statusText === "logged_in") {
      return "已登录";
    }
    if (status.statusText === "wait" || status.statusText === "scaned") return "等待扫码确认";
    return status.lastError || status.statusText || "未连接";
  }
  if (channel.platform === "dingtalk") {
    const status = channelRuntimeStates.value[channel.id];
    if (!channel.enabled) return t("config.remoteIm.disabledState");
    if (!status) return t("config.remoteIm.dingtalkConnectingState");
    if (status.connected) return t("config.remoteIm.connected");
    return t("config.remoteIm.dingtalkConnectingState");
  }
  if (channel.platform === "feishu") {
    return channel.enabled
      ? t("config.remoteIm.feishuSendOnlyState")
      : t("config.remoteIm.disabledState");
  }
  if (channel.platform !== "onebot_v11") {
    return channel.enabled ? t("config.remoteIm.enabledState") : t("config.remoteIm.disabledState");
  }
  const status = channelRuntimeStates.value[channel.id];
  if (!status) {
    return channel.enabled ? t("config.remoteIm.serverNotStarted") : t("config.remoteIm.disabledState");
  }
  if (status.connected) {
    return t("config.remoteIm.connected");
  }
  if (status.statusText === "binding_retry") return status.lastError || "固定端口被占用，正在重试";
  if (status.statusText === "binding") return "正在绑定固定端口";
  return status.listenAddr ? t("config.remoteIm.waitingForConnection") : t("config.remoteIm.serverNotStarted");
}

function channelListStatusBadgeText(channel: RemoteImChannelConfig): string {
  if (!channel.enabled) return t("config.remoteIm.disabledState");
  if (channel.platform === "onebot_v11" || channel.platform === "dingtalk" || channel.platform === "weixin_oc") {
    const status = channelRuntimeStates.value[channel.id];
    if (status?.connected) return t("config.remoteIm.connected");
    if (channel.platform === "onebot_v11" && status?.statusText === "binding_retry") return "重试绑定";
    if (channel.platform === "onebot_v11" && status?.statusText === "binding") return "绑定中";
    if (channel.platform === "weixin_oc" && status?.statusText === "need_login") return "待登录";
    return t("config.remoteIm.enabledState");
  }
  return t("config.remoteIm.enabledState");
}

function channelListStatusBadgeClass(channel: RemoteImChannelConfig): string {
  if (!channel.enabled) return "badge-ghost";
  if (channel.platform === "onebot_v11" || channel.platform === "dingtalk" || channel.platform === "weixin_oc") {
    const status = channelRuntimeStates.value[channel.id];
    if (channel.platform === "weixin_oc" && status?.statusText === "need_login") return "badge-warning";
    return status?.connected ? "badge-success" : "badge-warning";
  }
  return "badge-success";
}

async function refreshChannelLogs() {
  if (!selectedChannel.value) return;
  channelLogsLoading.value = true;
  try {
    channelLogs.value = await invokeTauri<ChannelLogEntry[]>("remote_im_get_channel_logs", {
      channelId: selectedChannel.value.id,
    });
  } catch {
    channelLogs.value = [];
  } finally {
    channelLogsLoading.value = false;
  }
}

async function refreshContactLogs() {
  if (!contactLogsContactId.value) return;
  contactLogsLoading.value = true;
  try {
    contactLogs.value = await invokeTauri<ChannelLogEntry[]>("remote_im_get_contact_logs", {
      input: { contactId: contactLogsContactId.value },
    });
  } catch {
    contactLogs.value = [];
  } finally {
    contactLogsLoading.value = false;
  }
}

function openChannelLogsModal() {
  if (!selectedChannel.value) return;
  channelLogsModalOpen.value = true;
  void refreshChannelLogs();
}

function openChannelLogsModalForChannel(channelId: string) {
  selectedChannelId.value = channelId;
  channelLogsModalOpen.value = true;
  void refreshChannelLogs();
}

function closeChannelLogsModal() {
  channelLogsModalOpen.value = false;
}

function openContactLogsModal(contactId: string) {
  contactLogsContactId.value = contactId;
  contactLogsModalOpen.value = true;
  void refreshContactLogs();
}

function closeContactLogsModal() {
  contactLogsModalOpen.value = false;
}

function openChannelConfigModal(channelId: string) {
  selectedChannelId.value = channelId;
  channelConfigModalOpen.value = true;
}

function closeChannelConfigModal() {
  channelConfigModalOpen.value = false;
}

function openContactConfigModal(contactId: string) {
  selectedContactId.value = contactId;
  syncSelectedContactDraft();
  contactConfigModalOpen.value = true;
}

function closeContactConfigModal() {
  contactConfigModalOpen.value = false;
  syncSelectedContactDraft();
}

watch(
  channels,
  (list) => {
    if (list.length > 0 && !list.some((ch) => ch.id === selectedChannelId.value)) {
      selectedChannelId.value = list[0].id;
    }
    for (const item of list) {
      if (!(item.id in credentialDrafts.value)) {
        credentialDrafts.value[item.id] = JSON.stringify(item.credentials || {}, null, 2);
      }
    }
  },
  { immediate: true },
);

watch(selectedChannelId, () => {
  if (selectedChannel.value) {
    credentialDrafts.value[selectedChannel.value.id] = JSON.stringify(
      selectedChannel.value.credentials || {}, null, 2,
    );
    if (selectedChannel.value.platform === "onebot_v11") {
      loadNapcatCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    } else if (selectedChannel.value.platform === "dingtalk") {
      loadDingtalkCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    } else if (selectedChannel.value.platform === "weixin_oc") {
      loadWeixinCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    } else {
      channelStatus.value = null;
    }
    if (channelLogsModalOpen.value) {
      void refreshChannelLogs();
    } else {
      channelLogs.value = [];
    }
  }
  lastSavedChannelSnapshot.value = channelSnapshot.value;
});

watch(napcatCredentials, () => {
  if (suppressCredentialSync.value) return;
  if (selectedChannel.value && selectedChannel.value.platform === "onebot_v11") {
    selectedChannel.value.credentials = {
      wsHost: napcatCredentials.value.wsHost || "0.0.0.0",
      wsPort: napcatCredentials.value.wsPort || 6199,
      wsToken: napcatCredentials.value.wsToken || "",
    };
  }
}, { deep: true });

watch(dingtalkCredentials, () => {
  if (suppressCredentialSync.value) return;
  if (selectedChannel.value && selectedChannel.value.platform === "dingtalk") {
    const current = selectedChannel.value.credentials || {};
    selectedChannel.value.credentials = {
      ...current,
      clientId: dingtalkCredentials.value.clientId || "",
      clientSecret: dingtalkCredentials.value.clientSecret || "",
    };
  }
}, { deep: true });

watch(weixinCredentials, () => {
  if (suppressCredentialSync.value) return;
  if (selectedChannel.value && selectedChannel.value.platform === "weixin_oc") {
    const current = selectedChannel.value.credentials || {};
    selectedChannel.value.credentials = {
      ...current,
      baseUrl: weixinCredentials.value.baseUrl || "https://ilinkai.weixin.qq.com",
      botType: WEIXIN_OC_BOT_TYPE,
      qrPollInterval: WEIXIN_OC_QR_POLL_INTERVAL,
      longPollTimeoutMs: WEIXIN_OC_LONG_POLL_TIMEOUT_MS,
      apiTimeoutMs: WEIXIN_OC_API_TIMEOUT_MS,
    };
  }
}, { deep: true });

onMounted(() => {
  if (channels.value.length > 0 && !selectedChannelId.value) {
    selectedChannelId.value = channels.value[0].id;
  }
  // channels watcher (immediate: true) 已在 selectedChannelId watcher 注册前
  // 就同步设置了 selectedChannelId，导致 selectedChannelId watcher 不会触发。
  // 这里需要手动执行初始化操作：加载 napcatCredentials 和刷新连接状态/日志。
  if (selectedChannel.value) {
    credentialDrafts.value[selectedChannel.value.id] = JSON.stringify(
      selectedChannel.value.credentials || {}, null, 2,
    );
    if (selectedChannel.value.platform === "onebot_v11") {
      loadNapcatCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    } else if (selectedChannel.value.platform === "dingtalk") {
      loadDingtalkCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    } else if (selectedChannel.value.platform === "weixin_oc") {
      loadWeixinCredentials(selectedChannel.value);
      channelStatus.value = channelRuntimeStates.value[selectedChannel.value.id] ?? null;
      void refreshChannelStatus();
    }
  }
  void refreshAllChannelStatuses();
  channelStatusTimer = setInterval(() => {
    void refreshAllChannelStatuses();
    if (channelLogsModalOpen.value) {
      void refreshChannelLogs();
    }
  }, 3000);
  lastSavedChannelSnapshot.value = channelSnapshot.value;
  void refreshContacts();
});

onUnmounted(() => {
  if (channelStatusTimer) {
    clearInterval(channelStatusTimer);
    channelStatusTimer = null;
  }
  if (weixinLoginPollTimer) {
    clearInterval(weixinLoginPollTimer);
    weixinLoginPollTimer = null;
  }
});
</script>
