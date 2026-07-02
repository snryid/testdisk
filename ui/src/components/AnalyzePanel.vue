<script setup>
import { t, filesystemLabel, partitionTableTypeLabel } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  scanResult: { type: Object, default: null },
  tableTypeLabel: { type: String, default: "-" },
  isScanning: { type: Boolean, default: false },
  formatSize: { type: Function, required: true },
  fsLabel: { type: Function, required: true },
  statusClass: { type: Function, required: true },
});

const emit = defineEmits(["scan"]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "analyze_panel_title") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "lost_hint") }}</p>
      </div>
      <button type="button" class="primary" :disabled="isScanning" @click="emit('scan')">
        {{ isScanning ? t(lang, "analyzing") : t(lang, "analyze") }}
      </button>
    </div>

    <div v-if="scanResult" class="scan-summary">
      <strong>{{ scanResult.disk_path }}</strong>
      <span>{{ t(lang, "size") }}: {{ formatSize(scanResult.disk_size) }}</span>
      <span>{{ t(lang, "partition_table") }}: {{ scanResult.partitions.length }}</span>
      <span>{{ t(lang, "lost_partitions") }}: {{ scanResult.lost_partitions.length }}</span>
    </div>

    <div v-if="scanResult?.warnings?.length" class="notice warning">
      <strong>{{ t(lang, "warnings") }}</strong>
      <div v-for="warning in scanResult.warnings" :key="warning">! {{ warning }}</div>
    </div>

    <div class="analysis-grid">
      <section class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "partition_table") }}</h3>
          <div class="badge">{{ tableTypeLabel }}</div>
        </div>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>{{ t(lang, "name") }}</th>
                <th>{{ t(lang, "start_lba") }}</th>
                <th>{{ t(lang, "partition_size") }}</th>
                <th>{{ t(lang, "partition_type") }}</th>
                <th>{{ t(lang, "fs_table") }}</th>
                <th>{{ t(lang, "status") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!scanResult?.partitions?.length" class="empty-row">
                <td colspan="7">{{ t(lang, "empty") }}</td>
              </tr>
              <tr v-for="partition in scanResult?.partitions" :key="partition.index">
                <td>{{ partition.index }}</td>
                <td>{{ partition.name }}</td>
                <td>{{ partition.start_lba }}</td>
                <td>{{ formatSize(partition.size_bytes) }}</td>
                <td>{{ partition.type_name }}</td>
                <td>
                  <span v-if="partition.filesystem" class="fs-tag">{{ fsLabel(partition) }}</span>
                  <span v-else>-</span>
                </td>
                <td :class="statusClass(partition.status)">{{ partition.status }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "lost_partitions") }}</h3>
          <span class="section-subtitle">{{ t(lang, "lost_hint") }}</span>
        </div>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>{{ t(lang, "name") }}</th>
                <th>{{ t(lang, "start_lba") }}</th>
                <th>{{ t(lang, "partition_size") }}</th>
                <th>{{ t(lang, "fs_table") }}</th>
                <th>{{ t(lang, "source_column") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!scanResult?.lost_partitions?.length" class="empty-row">
                <td colspan="6">{{ t(lang, "empty_lost") }}</td>
              </tr>
              <tr v-for="partition in scanResult?.lost_partitions" :key="partition.index">
                <td>{{ partition.index }}</td>
                <td>{{ partition.name }}</td>
                <td>{{ partition.start_lba }}</td>
                <td>{{ formatSize(partition.size_bytes) }}</td>
                <td><span class="fs-tag">{{ fsLabel(partition) }}</span></td>
                <td>{{ partition.source }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </section>
</template>
