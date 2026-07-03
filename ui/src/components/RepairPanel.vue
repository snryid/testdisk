<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  targetPath: { type: String, default: "" },
  diagnosisStatus: { type: String, default: "" },
  diagnoses: { type: Array, default: () => [] },
  repairPlans: { type: Array, default: () => [] },
  isDiagnosing: { type: Boolean, default: false },
  isPreviewing: { type: Boolean, default: false },
});

const emit = defineEmits([
  "update:targetPath",
  "use-selected-target",
  "diagnose",
  "preview-repair",
]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "tab_repair") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "repair_panel_hint") }}</p>
      </div>
      <div class="button-row compact-row">
        <button type="button" @click="emit('use-selected-target')">{{ t(lang, "use_selected_target") }}</button>
        <button type="button" class="primary" :disabled="isDiagnosing" @click="emit('diagnose')">
          {{ isDiagnosing ? t(lang, "diagnosing") : t(lang, "repair_diagnose") }}
        </button>
      </div>
    </div>

    <div class="workflow-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "boot_sector") }}</h3>
          <span class="section-subtitle">{{ diagnosisStatus || t(lang, "repair_ready") }}</span>
        </div>
        <label class="field">
          <span>{{ t(lang, "target_path") }}</span>
          <input
            :value="targetPath"
            type="text"
            :placeholder="selectedDisk?.path || t(lang, 'choose_disk')"
            @input="emit('update:targetPath', $event.target.value)"
          />
        </label>
        <div class="button-row">
          <button type="button" :disabled="isPreviewing" @click="emit('preview-repair')">
            {{ isPreviewing ? t(lang, "previewing") : t(lang, "repair_preview") }}
          </button>
        </div>

        <div v-if="repairPlans.length" class="stack-list">
          <article v-for="plan in repairPlans" :key="`${plan.target_path}-${plan.filesystem}`" class="inner-card">
            <div class="section-head">
              <h4>{{ plan.filesystem }}</h4>
              <span class="section-subtitle">{{ plan.target_path }}</span>
            </div>
            <div class="key-value-grid compact">
              <div>
                <span>{{ t(lang, "boot_primary_lba") }}</span>
                <strong>{{ plan.primary_lba }}</strong>
              </div>
              <div>
                <span>{{ t(lang, "boot_backup_lba") }}</span>
                <strong>{{ plan.backup_lba ?? "-" }}</strong>
              </div>
            </div>
            <div v-if="plan.native_commands?.length" class="command-list">
              <div v-for="command in plan.native_commands" :key="command.program" class="command-chip">
                <strong>{{ command.program }}</strong>
                <span>{{ command.args.join(" ") }}</span>
              </div>
            </div>
          </article>
        </div>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "boot_sector_diagnosis") }}</h3>
          <span class="section-subtitle">{{ t(lang, "repair_diagnosis_hint") }}</span>
        </div>
        <div v-if="diagnoses.length" class="stack-list">
          <article v-for="diagnosis in diagnoses" :key="`${diagnosis.filesystem}-${diagnosis.primary_lba}`" class="inner-card">
            <div class="section-head">
              <h4>{{ diagnosis.filesystem }}</h4>
              <span class="section-subtitle">{{ diagnosis.confidence }}</span>
            </div>
            <div class="key-value-grid compact">
              <div>
                <span>{{ t(lang, "boot_primary_lba") }}</span>
                <strong>{{ diagnosis.primary_lba }}</strong>
              </div>
              <div>
                <span>{{ t(lang, "boot_backup_lba") }}</span>
                <strong>{{ diagnosis.backup_lba ?? "-" }}</strong>
              </div>
              <div>
                <span>{{ t(lang, "match_backup") }}</span>
                <strong>{{ diagnosis.matches_backup ? t(lang, "yes") : t(lang, "no") }}</strong>
              </div>
            </div>
            <ul class="compact-list">
              <li v-for="finding in diagnosis.findings" :key="finding">{{ finding }}</li>
            </ul>
          </article>
        </div>
        <p v-else class="muted-block">{{ t(lang, "repair_empty") }}</p>
      </article>
    </div>
  </section>
</template>
