<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  sourcePath: { type: String, default: "" },
  destinationDir: { type: String, default: "" },
  recoveryStatus: { type: String, default: "" },
  recoveryTranscript: { type: Object, default: null },
  isRecovering: { type: Boolean, default: false },
});

const emit = defineEmits([
  "update:sourcePath",
  "update:destinationDir",
  "use-selected-target",
  "choose-destination",
  "recover",
]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "tab_recovery") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "recovery_panel_hint") }}</p>
      </div>
      <div class="button-row compact-row">
        <button type="button" @click="emit('use-selected-target')">{{ t(lang, "use_selected_target") }}</button>
        <button type="button" class="primary" :disabled="isRecovering" @click="emit('recover')">
          {{ isRecovering ? t(lang, "recovering") : t(lang, "recover_files") }}
        </button>
      </div>
    </div>

    <div class="workflow-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "undelete_title") }}</h3>
          <span class="section-subtitle">{{ recoveryStatus || t(lang, "recovery_ready") }}</span>
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
          <span>{{ t(lang, "destination_dir") }}</span>
          <div class="inline-field">
            <input :value="destinationDir" type="text" @input="emit('update:destinationDir', $event.target.value)" />
            <button type="button" @click="emit('choose-destination')">{{ t(lang, "browse") }}</button>
          </div>
        </label>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "recovery_result") }}</h3>
          <span class="section-subtitle">{{ recoveryTranscript?.status || t(lang, "recovery_empty") }}</span>
        </div>
        <div v-if="recoveryTranscript" class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "recovery_recovered_count") }}</span>
            <strong>{{ recoveryTranscript.recovered_files.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "source_path") }}</span>
            <strong>{{ recoveryTranscript.source_path }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "destination_dir") }}</span>
            <strong>{{ recoveryTranscript.destination_dir }}</strong>
          </div>
        </div>
        <div v-if="recoveryTranscript?.recovered_files?.length" class="stack-list">
          <article
            v-for="file in recoveryTranscript.recovered_files"
            :key="file.destination_path"
            class="inner-card"
          >
            <div class="section-head">
              <h4>{{ file.destination_path.split('/').pop() }}</h4>
              <span class="section-subtitle">{{ file.status }}</span>
            </div>
            <div class="compact-metadata">
              <span>{{ file.bytes_written }} B</span>
              <span>{{ file.note }}</span>
            </div>
          </article>
        </div>
        <p v-else class="muted-block">{{ t(lang, "recovery_empty") }}</p>
      </article>
    </div>
  </section>
</template>
