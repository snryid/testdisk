<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  sourcePath: { type: String, default: "" },
  outputPath: { type: String, default: "" },
  imagingStatus: { type: String, default: "" },
  imagingTranscript: { type: Object, default: null },
  isImaging: { type: Boolean, default: false },
});

const emit = defineEmits([
  "update:sourcePath",
  "update:outputPath",
  "use-selected-target",
  "choose-output",
  "image",
]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "tab_imaging") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "imaging_panel_hint") }}</p>
      </div>
      <div class="button-row compact-row">
        <button type="button" @click="emit('use-selected-target')">{{ t(lang, "use_selected_target") }}</button>
        <button type="button" class="primary" :disabled="isImaging" @click="emit('image')">
          {{ isImaging ? t(lang, "imaging") : t(lang, "image_disk") }}
        </button>
      </div>
    </div>

    <div class="workflow-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "image_job") }}</h3>
          <span class="section-subtitle">{{ imagingStatus || t(lang, "imaging_ready") }}</span>
        </div>
        <label class="field">
          <span>{{ t(lang, "source_path") }}</span>
          <input
            :value="sourcePath"
            type="text"
            :placeholder="selectedDisk?.path || t(lang, 'choose_disk')"
            @input="emit('update:sourcePath', $event.target.value)"
          />
        </label>
        <label class="field">
          <span>{{ t(lang, "output_path") }}</span>
          <div class="inline-field">
            <input :value="outputPath" type="text" @input="emit('update:outputPath', $event.target.value)" />
            <button type="button" @click="emit('choose-output')">{{ t(lang, "browse") }}</button>
          </div>
        </label>
        <div class="notice info">
          <strong>{{ t(lang, "imaging_triage_title") }}</strong>
          <p>{{ t(lang, "imaging_triage_body") }}</p>
        </div>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "imaging_result") }}</h3>
          <span class="section-subtitle">{{ imagingTranscript?.status || t(lang, "imaging_empty") }}</span>
        </div>
        <div v-if="imagingTranscript" class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "bytes_copied") }}</span>
            <strong>{{ imagingTranscript.bytes_copied }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "checksum") }}</span>
            <strong>{{ imagingTranscript.checksum }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "read_errors") }}</span>
            <strong>{{ imagingTranscript.read_errors }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "throughput") }}</span>
            <strong>{{ imagingTranscript.throughput_bytes_per_sec }} B/s</strong>
          </div>
        </div>
        <p v-else class="muted-block">{{ t(lang, "imaging_empty") }}</p>
      </article>
    </div>
  </section>
</template>
