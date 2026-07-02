<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  scanResult: { type: Object, default: null },
  exportStatus: { type: String, default: "" },
  formatSize: { type: Function, required: true },
});

const emit = defineEmits(["export-report"]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "reports_panel_title") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "report_panel_title") }}</p>
      </div>
      <button type="button" @click="emit('export-report')">{{ t(lang, "report_export") }}</button>
    </div>

    <div class="report-stack">
      <div v-if="scanResult" class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "report_panel_title") }}</h3>
          <span class="section-subtitle">{{ scanResult.disk_path }}</span>
        </div>
        <div class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "size") }}</span>
            <strong>{{ formatSize(scanResult.disk_size) }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "partition_table") }}</span>
            <strong>{{ scanResult.partitions.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "lost_partitions") }}</span>
            <strong>{{ scanResult.lost_partitions.length }}</strong>
          </div>
        </div>
        <div v-if="scanResult.warnings?.length" class="notice warning">
          <strong>{{ t(lang, "warnings") }}</strong>
          <div v-for="warning in scanResult.warnings" :key="warning">! {{ warning }}</div>
        </div>
      </div>

      <div v-if="exportStatus" class="notice info">
        <strong>{{ t(lang, "report_export") }}</strong>
        <p>{{ exportStatus }}</p>
      </div>
    </div>
  </section>
</template>
