import { computed, ref } from "vue";
import type { AgentWorkSignalPayload } from "../../../types/app";

const AGENT_WORK_TTL_MS = 10 * 60 * 1000;
const AGENT_WORK_PRUNE_INTERVAL_MS = 30 * 1000;

type AgentWorkRegistry = Record<string, Record<string, Record<string, number>>>;

function trim_text(value: unknown): string {
  return String(value || "").trim();
}

function pruneExpiredRegistry(source: AgentWorkRegistry, now: number): AgentWorkRegistry {
  const next: AgentWorkRegistry = {};
  for (const [conversationId, agents] of Object.entries(source)) {
    const nextAgents: Record<string, Record<string, number>> = {};
    for (const [agentId, jobs] of Object.entries(agents)) {
      const aliveJobs = Object.entries(jobs).filter(([, expiresAt]) => Number(expiresAt) > now);
      if (aliveJobs.length <= 0) continue;
      nextAgents[agentId] = Object.fromEntries(aliveJobs);
    }
    if (Object.keys(nextAgents).length <= 0) continue;
    next[conversationId] = nextAgents;
  }
  return next;
}

function hasAnyActiveJob(source: AgentWorkRegistry): boolean {
  return Object.values(source).some((agents) =>
    Object.values(agents).some((jobs) => Object.keys(jobs).length > 0),
  );
}

export function useAgentWorkPresence() {
  const workRegistry = ref<AgentWorkRegistry>({});
  let pruneTimer: ReturnType<typeof window.setInterval> | null = null;

  function commitRegistry(next: AgentWorkRegistry) {
    workRegistry.value = next;
    if (hasAnyActiveJob(next)) {
      if (pruneTimer === null) {
        pruneTimer = window.setInterval(() => {
          const pruned = pruneExpiredRegistry(workRegistry.value, Date.now());
          if (pruned !== workRegistry.value) {
            workRegistry.value = pruned;
          }
          if (!hasAnyActiveJob(pruned) && pruneTimer !== null) {
            window.clearInterval(pruneTimer);
            pruneTimer = null;
          }
        }, AGENT_WORK_PRUNE_INTERVAL_MS);
      }
    } else if (pruneTimer !== null) {
      window.clearInterval(pruneTimer);
      pruneTimer = null;
    }
  }

  function markAgentWorkStarted(payload: AgentWorkSignalPayload) {
    const conversationId = trim_text(payload.conversationId);
    const agentId = trim_text(payload.agentId);
    const delegateId = trim_text(payload.delegateId);
    if (!conversationId || !agentId || !delegateId) return;
    const now = Date.now();
    const next = pruneExpiredRegistry(workRegistry.value, now);
    next[conversationId] = next[conversationId] || {};
    next[conversationId][agentId] = {
      ...(next[conversationId][agentId] || {}),
      [delegateId]: now + AGENT_WORK_TTL_MS,
    };
    commitRegistry(next);
  }

  function markAgentWorkStopped(payload: AgentWorkSignalPayload) {
    const conversationId = trim_text(payload.conversationId);
    const agentId = trim_text(payload.agentId);
    const delegateId = trim_text(payload.delegateId);
    if (!conversationId || !agentId || !delegateId) return;
    const next = pruneExpiredRegistry(workRegistry.value, Date.now());
    if (!next[conversationId] || !next[conversationId][agentId] || !(delegateId in next[conversationId][agentId])) {
      commitRegistry(next);
      return;
    }
    delete next[conversationId][agentId][delegateId];
    if (Object.keys(next[conversationId][agentId]).length <= 0) {
      delete next[conversationId][agentId];
    }
    if (Object.keys(next[conversationId]).length <= 0) {
      delete next[conversationId];
    }
    commitRegistry(next);
  }

  const activeWorkCountsByConversation = computed<Record<string, Record<string, number>>>(() => {
    const out: Record<string, Record<string, number>> = {};
    for (const [conversationId, agents] of Object.entries(workRegistry.value)) {
      const conversationCounts: Record<string, number> = {};
      for (const [agentId, jobs] of Object.entries(agents)) {
        const count = Object.keys(jobs).length;
        if (count > 0) conversationCounts[agentId] = count;
      }
      if (Object.keys(conversationCounts).length > 0) {
        out[conversationId] = conversationCounts;
      }
    }
    return out;
  });

  function activeWorkCountForAgent(conversationId: string, agentId: string): number {
    const normalizedConversationId = trim_text(conversationId);
    const normalizedAgentId = trim_text(agentId);
    return Math.max(0, Number(activeWorkCountsByConversation.value[normalizedConversationId]?.[normalizedAgentId] || 0));
  }

  function cleanup() {
    if (pruneTimer !== null) {
      window.clearInterval(pruneTimer);
      pruneTimer = null;
    }
    workRegistry.value = {};
  }

  return {
    activeWorkCountsByConversation,
    activeWorkCountForAgent,
    markAgentWorkStarted,
    markAgentWorkStopped,
    cleanup,
  };
}
