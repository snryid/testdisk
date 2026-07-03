<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  sourcePath: { type: String, default: "" },
  outputDir: { type: String, default: "" },
  selectedFamilies: { type: Array, default: () => [] },
  carvingStatus: { type: String, default: "" },
  carvingTranscript: { type: Object, default: null },
  isCarving: { type: Boolean, default: false },
});

const emit = defineEmits([
  "update:sourcePath",
  "update:outputDir",
  "update:selectedFamilies",
  "use-selected-target",
  "choose-output",
  "carve",
]);

const familyOptions = ["jpeg", "png", "pdf"];
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "tab_carving") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "carving_panel_hint") }}</p>
      </div>
      <div class="button-row compact-row">
        <button type="button" @click="emit('use-selected-target')">{{ t(lang, "use_selected_target") }}</button>
        <button type="button" class="primary" :disabled="isCarving" @click="emit('carve')">
          {{ isCarving ? t(lang, "carving") : t(lang, "carve_files") }}
        </button>
      </div>
    </div>

    <div class="workflow-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "carving_job") }}</h3>
          <span class="section-subtitle">{{ carvingStatus || t(lang, "carving_ready") }}</span>
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
          <span>{{ t(lang, "output_dir") }}</span>
          <div class="inline-field">
            <input :value="outputDir" type="text" @input="emit('update:outputDir', $event.target.value)" />
            <button type="button" @click="emit('choose-output')">{{ t(lang, "browse") }}</button>
          </div>
        </label>
        <div class="chip-grid">
          <label v-for="family in familyOptions" :key="family" class="chip-option">
            <input
              :checked="selectedFamilies.includes(family)"
              type="checkbox"
              @change="
                emit(
                  'update:selectedFamilies',
                  $event.target.checked
                    ? [...selectedFamilies, family]
                    : selectedFamilies.filter((value) => value !== family)
                )
              "
            />
            <span>{{ family }}</span>
          </label>
        </div>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "carving_result") }}</h3>
          <span class="section-subtitle">{{ carvingTranscript?.status || t(lang, "carving_empty") }}</span>
        </div>
        <div v-if="carvingTranscript" class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "carving_count") }}</span>
            <strong>{{ carvingTranscript.carved_files.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "source_path") }}</span>
            <strong>{{ carvingTranscript.source_path }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "output_dir") }}</span>
            <strong>{{ carvingTranscript.output_dir }}</strong>
          </div>
        </div>
        <div v-if="carvingTranscript?.carved_files?.length" class="stack-list">
          <article v-for="file in carvingTranscript.carved_files" :key="file.output_path" class="inner-card">
            <div class="section-head">
              <h4>{{ file.family_id }}</h4>
              <span class="section-subtitle">{{ file.status }}</span>
            </div>
            <div class="compact-metadata">
              <span>{{ file.output_path }}</span>
              <span>{{ file.length }} B</span>
            </div>
          </article>
        </div>
        <p v-else class="muted-block">{{ t(lang, "carving_empty") }}</p>
      </article>
    </div>
  </section>
</template>
