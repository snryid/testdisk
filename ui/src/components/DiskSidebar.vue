<script setup>
import { t } from "../i18n";

defineProps({
  diskOptions: { type: Array, required: true },
  selectedPath: { type: String, default: "" },
  selectedDisk: { type: Object, default: null },
  selectedDiskStateNotice: { type: Object, default: null },
  selectedDiskSourceLabel: { type: String, default: "-" },
  selectedDiskAccessLabel: { type: String, default: "-" },
  selectedDiskSafetyLabel: { type: String, default: "-" },
  isLoadingDisks: { type: Boolean, default: false },
  loadError: { type: String, default: "" },
  listStateNotice: { type: Object, default: null },
  readableDiskCount: { type: Number, default: 0 },
  hasAnyDisks: { type: Boolean, default: false },
  lang: { type: String, required: true },
  formatSize: { type: Function, required: true },
  diskOptionLabel: { type: Function, required: true },
});

const emit = defineEmits(["update:selectedPath", "refresh", "open-image"]);
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-section">
      <div class="section-head">
        <h2>{{ t(lang, "sidebar_title") }}</h2>
        <span class="section-subtitle">{{ t(lang, "select_disk_label") }}</span>
      </div>
      <label class="field">
        <span>{{ t(lang, "select_disk_label") }}</span>
        <select
          :value="selectedPath"
          :disabled="isLoadingDisks"
          @change="emit('update:selectedPath', $event.target.value)"
        >
          <option value="">
            {{
              isLoadingDisks
                ? t(lang, "loading")
                : !hasAnyDisks
                  ? t(lang, "no_disk")
                  : readableDiskCount === 0
                    ? t(lang, "no_readable_disk")
                    : t(lang, "choose_disk")
            }}
          </option>
          <option v-for="disk in diskOptions" :key="disk.path" :value="disk.path">
            {{ diskOptionLabel(disk) }}
          </option>
        </select>
      </label>
      <div class="button-row">
        <button type="button" @click="emit('refresh')">{{ t(lang, "refresh_disks") }}</button>
        <button type="button" @click="emit('open-image')">{{ t(lang, "open_image") }}</button>
      </div>
    </div>

    <div v-if="loadError" class="notice danger">
      <strong>{{ t(lang, "load_failed_title") }}</strong>
      <p>{{ loadError }}</p>
    </div>

    <div v-else-if="listStateNotice" :class="['notice', listStateNotice.variant]">
      <strong>{{ listStateNotice.title }}</strong>
      <p>{{ listStateNotice.body }}</p>
      <p v-if="!hasAnyDisks" class="notice-hint">{{ t(lang, "empty_list_hint") }}</p>
    </div>

    <div class="sidebar-scroll">
      <section v-if="selectedDisk" class="target-card">
        <div class="section-head">
          <h3>{{ t(lang, "target_title") }}</h3>
          <span class="section-subtitle">{{ t(lang, "selected_disk_prefix") }}</span>
        </div>
        <div class="key-value-grid">
          <div>
            <span>{{ t(lang, "platform_id") }}</span>
            <strong>{{ selectedDisk.platform_id || "-" }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "display_path") }}</span>
            <strong>{{ selectedDisk.display_path || selectedDisk.path }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "raw_path") }}</span>
            <strong>{{ selectedDisk.raw_path || selectedDisk.path }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "source") }}</span>
            <strong>{{ selectedDiskSourceLabel }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "protocol") }}</span>
            <strong>{{ selectedDisk.protocol || "-" }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "access") }}</span>
            <strong>{{ selectedDiskAccessLabel }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "safety") }}</span>
            <strong>{{ selectedDiskSafetyLabel }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "size") }}</span>
            <strong>{{ formatSize(selectedDisk.size_bytes) }}</strong>
          </div>
        </div>
        <div v-if="selectedDiskStateNotice" :class="['notice', selectedDiskStateNotice.variant]">
          <strong>{{ selectedDiskStateNotice.title }}</strong>
          <p>{{ selectedDiskStateNotice.body }}</p>
        </div>
      </section>
    </div>
  </aside>
</template>
