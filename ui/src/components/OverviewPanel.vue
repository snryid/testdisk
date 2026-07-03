<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  runtimeLabel: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  selectedDiskSourceLabel: { type: String, default: "-" },
  selectedDiskAccessLabel: { type: String, default: "-" },
  selectedDiskSafetyLabel: { type: String, default: "-" },
  scanResult: { type: Object, default: null },
  formatSize: { type: Function, required: true },
  formatStatus: { type: String, default: "" },
  repairStatus: { type: String, default: "" },
  recoveryStatus: { type: String, default: "" },
  imagingStatus: { type: String, default: "" },
  carvingStatus: { type: String, default: "" },
  automationStatus: { type: String, default: "" },
});

const emit = defineEmits(["go-tab"]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "overview_panel_title") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "overview_panel_hint") }}</p>
      </div>
      <span class="badge">{{ runtimeLabel }}</span>
    </div>

    <div class="overview-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "target_title") }}</h3>
          <span class="section-subtitle">{{ t(lang, "selected_disk_prefix") }}</span>
        </div>
        <div v-if="selectedDisk" class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "display_path") }}</span>
            <strong>{{ selectedDisk.display_path || selectedDisk.path }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "platform_id") }}</span>
            <strong>{{ selectedDisk.platform_id || "-" }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "source") }}</span>
            <strong>{{ selectedDiskSourceLabel }}</strong>
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
        <p v-else class="muted-block">{{ t(lang, "overview_no_target") }}</p>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "overview_workflows") }}</h3>
          <span class="section-subtitle">{{ t(lang, "overview_workflows_hint") }}</span>
        </div>
        <div class="workflow-status-list">
          <div><span>{{ t(lang, "tab_format") }}</span><strong>{{ formatStatus || "-" }}</strong></div>
          <div><span>{{ t(lang, "tab_repair") }}</span><strong>{{ repairStatus || "-" }}</strong></div>
          <div><span>{{ t(lang, "tab_recovery") }}</span><strong>{{ recoveryStatus || "-" }}</strong></div>
          <div><span>{{ t(lang, "tab_imaging") }}</span><strong>{{ imagingStatus || "-" }}</strong></div>
          <div><span>{{ t(lang, "tab_carving") }}</span><strong>{{ carvingStatus || "-" }}</strong></div>
          <div><span>{{ t(lang, "tab_automation") }}</span><strong>{{ automationStatus || "-" }}</strong></div>
        </div>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "overview_scan") }}</h3>
          <span class="section-subtitle">{{ t(lang, "overview_scan_hint") }}</span>
        </div>
        <div v-if="scanResult" class="key-value-grid compact">
          <div>
            <span>{{ t(lang, "partition_table") }}</span>
            <strong>{{ scanResult.partitions.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "lost_partitions") }}</span>
            <strong>{{ scanResult.lost_partitions.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "warnings") }}</span>
            <strong>{{ scanResult.warnings.length }}</strong>
          </div>
          <div>
            <span>{{ t(lang, "size") }}</span>
            <strong>{{ formatSize(scanResult.disk_size) }}</strong>
          </div>
        </div>
        <p v-else class="muted-block">{{ t(lang, "overview_no_scan") }}</p>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "overview_quick_start") }}</h3>
          <span class="section-subtitle">{{ t(lang, "overview_quick_start_hint") }}</span>
        </div>
        <div class="quick-actions">
          <button type="button" class="primary" @click="emit('go-tab', 'analyze')">{{ t(lang, "tab_analyze") }}</button>
          <button type="button" @click="emit('go-tab', 'format')">{{ t(lang, "tab_format") }}</button>
          <button type="button" @click="emit('go-tab', 'repair')">{{ t(lang, "tab_repair") }}</button>
          <button type="button" @click="emit('go-tab', 'recovery')">{{ t(lang, "tab_recovery") }}</button>
          <button type="button" @click="emit('go-tab', 'imaging')">{{ t(lang, "tab_imaging") }}</button>
          <button type="button" @click="emit('go-tab', 'automation')">{{ t(lang, "tab_automation") }}</button>
        </div>
      </article>
    </div>
  </section>
</template>
