<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

type SshStatus = "connected" | "connecting" | "disconnected" | "error";

const props = withDefaults(
  defineProps<{
    status: SshStatus;
    size?: number;
    showLabel?: boolean;
  }>(),
  {
    size: 8,
    showLabel: false,
  },
);

const { t } = useI18n();

const colorMap: Record<SshStatus, string> = {
  connected: "#00b42a",
  connecting: "#ff7d00",
  disconnected: "#86909c",
  error: "#f53f3f",
};

const dotColor = computed(() => colorMap[props.status]);

const label = computed(() => {
  const map: Record<SshStatus, string> = {
    connected: t("remote.statusConnected"),
    connecting: t("remote.statusConnecting"),
    disconnected: t("remote.statusDisconnected"),
    error: t("remote.statusError"),
  };
  return map[props.status];
});

const isPulsing = computed(() => props.status === "connecting");
</script>

<template>
  <span class="ssh-status-indicator" :title="label">
    <span
      class="status-dot"
      :class="{ pulsing: isPulsing }"
      :style="{
        width: `${size}px`,
        height: `${size}px`,
        backgroundColor: dotColor,
      }"
    />
    <span v-if="showLabel" class="status-label">{{ label }}</span>
  </span>
</template>

<style scoped>
.ssh-status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  display: inline-block;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.pulsing {
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.status-label {
  font-size: 12px;
  color: var(--color-text-2);
}
</style>
