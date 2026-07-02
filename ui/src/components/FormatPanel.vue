<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  localizedFormatFilesystems: { type: Array, required: true },
  selectedFormatFilesystem: { type: String, required: true },
  formatVolumeName: { type: String, required: true },
  formatConfirmation: { type: String, required: true },
  formatConfirmationTarget: { type: String, default: "" },
  formatStatus: { type: String, default: "" },
  formatBlockReason: { type: String, default: "" },
  canFormat: { type: Boolean, default: false },
  isFormatting: { type: Boolean, default: false },
});

const emit = defineEmits([
  "update:selectedFormatFilesystem",
  "update:formatVolumeName",
  "update:formatConfirmation",
  "format",
]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "format_panel_title") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "format_hint") }}</p>
      </div>
      <span class="danger-chip">{{ t(lang, "format_title") }}</span>
    </div>

    <div class="format-workflow">
      <div class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "format_title") }}</h3>
          <span class="section-subtitle">{{ formatBlockReason }}</span>
        </div>
        <div class="format-grid">
          <label class="field">
            <span>{{ t(lang, "filesystem") }}</span>
            <select
              :value="selectedFormatFilesystem"
              @change="emit('update:selectedFormatFilesystem', $event.target.value)"
            >
              <option
                v-for="filesystem in localizedFormatFilesystems"
                :key="filesystem.value"
                :value="filesystem.value"
              >
                {{ filesystem.label }}
              </option>
            </select>
          </label>
          <label class="field">
            <span>{{ t(lang, "volume_name") }}</span>
            <input
              :value="formatVolumeName"
              type="text"
              maxlength="32"
              placeholder="UNTITLED"
              @input="emit('update:formatVolumeName', $event.target.value)"
            />
          </label>
          <label class="field">
            <span>{{ t(lang, "confirm_target") }}</span>
            <input
              :value="formatConfirmation"
              type="text"
              :placeholder="formatConfirmationTarget || t(lang, 'choose_disk')"
              @input="emit('update:formatConfirmation', $event.target.value)"
            />
          </label>
        </div>
        <div class="button-row danger-row">
          <button
          type="button"
          class="danger"
          :disabled="!canFormat || isFormatting"
          @click="emit('format')"
        >
            {{ isFormatting ? t(lang, "formatting") : t(lang, "format_disk") }}
          </button>
        </div>
        <p class="hint">
          {{ t(lang, "format_hint") }}
          <strong>{{ formatConfirmationTarget || "-" }}</strong>
        </p>
        <p v-if="formatStatus" class="format-status">{{ formatStatus }}</p>
      </div>

      <div v-if="selectedDisk" class="data-card summary-card">
        <div class="section-head">
          <h3>{{ t(lang, "target_title") }}</h3>
          <span class="section-subtitle">{{ t(lang, "selected_disk_prefix") }}</span>
        </div>
        <div class="key-value-grid compact">
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
            <span>{{ t(lang, "access") }}</span>
            <strong>{{ selectedDisk.access || "-" }}</strong>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
