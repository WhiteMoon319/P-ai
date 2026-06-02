import { ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";

export function useChatMessageActions() {
  const playingAudioId = ref("");
  let activeAudio: HTMLAudioElement | null = null;

  async function copyMessage(block: ChatMessageBlock) {
    const copyText = block.text || "";
    if (!copyText) return;
    try {
      await navigator.clipboard.writeText(copyText);
    } catch {
      // Ignore clipboard failures to avoid interrupting chat flow.
    }
  }

  function buildAudioDataUrl(audio: { mime: string; bytesBase64: string }): string {
    return `data:${audio.mime};base64,${audio.bytesBase64}`;
  }

  function stopAudioPlayback() {
    if (activeAudio) {
      activeAudio.pause();
      activeAudio.currentTime = 0;
      activeAudio = null;
    }
    playingAudioId.value = "";
  }

  function toggleAudioPlayback(id: string, audio: { mime: string; bytesBase64: string }) {
    if (playingAudioId.value === id && activeAudio) {
      stopAudioPlayback();
      return;
    }
    stopAudioPlayback();
    const player = new Audio(buildAudioDataUrl(audio));
    activeAudio = player;
    playingAudioId.value = id;
    player.onended = () => {
      if (activeAudio === player) {
        activeAudio = null;
        playingAudioId.value = "";
      }
    };
    void player.play().catch(() => {
      if (activeAudio === player) {
        activeAudio = null;
        playingAudioId.value = "";
      }
    });
  }

  return {
    playingAudioId,
    copyMessage,
    stopAudioPlayback,
    toggleAudioPlayback,
  };
}
