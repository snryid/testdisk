<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { filesystemLabel, partitionTableTypeLabel, t } from "./i18n";

const THEME_STORAGE_KEY = "mini-testdisk.theme";
const LANG_STORAGE_KEY = "mini-testdisk.lang";

const diskOptions = ref([]);
const formatFilesystems = ref([]);
const selectedPath = ref("");
const scanResult = ref(null);
const isLoadingDisks = ref(false);
const isScanning = ref(false);
const isFormatting = ref(false);
const loadError = ref("");
const selectedFormatFilesystem = ref("exfat");
const formatVolumeName = ref("UNTITLED");
const formatConfirmation = ref("");
const formatStatus = ref("");
const exportStatus = ref("");

const theme = ref(loadPreference(THEME_STORAGE_KEY, prefersDarkTheme() ? "dark" : "light"));
const lang = ref(loadPreference(LANG_STORAGE_KEY, "zh"));

const readableDiskCount = computed(() => diskOptions.value.filter((disk) => disk.readable).length);
const hasUnreadableSystemDisks = computed(() =>
  diskOptions.value.some((disk) => !disk.readable && disk.source === "system")
);
const showMacDiskPermissionNotice = computed(() =>
  hasUnreadableSystemDisks.value && readableDiskCount.value === 0
);
const selectedDisk = computed(() => diskOptions.value.find((disk) => disk.path === selectedPath.value));
const selectedDiskAccessLabel = computed(() => {
  const access = selectedDisk.value?.access;
  return access ? t(lang.value, `access_${access}`) : "-";
});
const selectedDiskSafetyLabel = computed(() => {
  const safety = selectedDisk.value?.safety;
  return safety ? t(lang.value, `safety_${safety}`) : "-";
});
const selectedDiskSourceLabel = computed(() =>
  selectedDisk.value?.source === "image_file"
    ? t(lang.value, "source_image")
    : selectedDisk.value
      ? t(lang.value, "source_system")
      : "-"
);
const formatConfirmationTarget = computed(() => selectedDisk.value?.path || "");
const tableTypeLabel = computed(() => {
  const type = scanResult.value?.partition_table_type;
  return type ? partitionTableTypeLabel(lang.value, type) : "-";
});
const localizedFormatFilesystems = computed(() =>
  formatFilesystems.value.map((filesystem) => ({
    ...filesystem,
    label: filesystemLabel(lang.value, filesystem.value),
  }))
);
const canFormat = computed(() =>
  Boolean(
    selectedDisk.value?.source === "system" &&
      selectedPath.value &&
      selectedFormatFilesystem.value &&
      formatVolumeName.value.trim()
  )
);

watch(theme, applyTheme, { immediate: true });
watch(lang, applyLanguage, { immediate: true });

function loadPreference(key, fallback) {
  try {
    return window.localStorage.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}

function savePreference(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    return;
  }
}

function prefersDarkTheme() {
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ?? true;
}

function applyTheme(value) {
  document.documentElement.dataset.theme = value;
  savePreference(THEME_STORAGE_KEY, value);
}

function applyLanguage(value) {
  document.documentElement.lang = value === "zh" ? "zh-CN" : "en";
  savePreference(LANG_STORAGE_KEY, value);
}

function setTheme(value) {
  theme.value = value;
}

function setLanguage(value) {
  lang.value = value;
}

function formatSize(bytes) {
  if (!bytes) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let index = 0;
  let size = bytes;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  return `${size.toFixed(index > 0 ? 1 : 0)} ${units[index]}`;
}

function fsLabel(partition) {
  return partition.filesystem?.fs_type || "-";
}

function statusClass(status) {
  if (status === "deleted") return "status-deleted";
  if (status === "bootable") return "status-bootable";
  return "";
}

async function loadDisks() {
  isLoadingDisks.value = true;
  loadError.value = "";
  try {
    diskOptions.value = await invoke("get_disks");
  } catch (error) {
    loadError.value = t(lang.value, "loading_disks", { error });
    diskOptions.value = [];
  } finally {
    isLoadingDisks.value = false;
  }
}

async function loadFormatFilesystems() {
  try {
    formatFilesystems.value = await invoke("get_format_filesystems");
  } catch (error) {
    formatStatus.value = t(lang.value, "loading_format_options", { error });
  }
}

async function openImage() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Disk Image", extensions: ["img", "dmg", "iso", "raw", "bin"] }],
  });
  if (!selected) return;

  try {
    const info = await invoke("open_image", { path: selected });
    diskOptions.value = [
      ...diskOptions.value.filter((disk) => disk.path !== info.path),
      info,
    ];
    selectedPath.value = info.path;
  } catch (error) {
    window.alert(t(lang.value, "open_image_failed", { error }));
  }
}

async function scanSelected() {
  if (!selectedPath.value) {
    window.alert(t(lang.value, "choose_target_first"));
    return;
  }

  isScanning.value = true;
  try {
    scanResult.value = await invoke("scan_disk_path", { path: selectedPath.value });
  } catch (error) {
    window.alert(t(lang.value, "scan_failed", { error }));
  } finally {
    isScanning.value = false;
  }
}

async function formatSelectedDisk() {
  if (!canFormat.value) {
    window.alert(t(lang.value, "choose_format_first"));
    return;
  }
  if (formatConfirmation.value.trim() !== formatConfirmationTarget.value) {
    window.alert(
      t(lang.value, "confirm_target_mismatch", {
        target: formatConfirmationTarget.value,
      })
    );
    return;
  }

  const filesystem = selectedFormatFilesystem.value;
  const volumeName = formatVolumeName.value.trim();
  const confirmed = window.confirm(
    t(lang.value, "confirm_format", {
      path: selectedPath.value,
      filesystem: filesystemLabel(lang.value, filesystem),
      volumeName,
    })
  );
  if (!confirmed) return;

  isFormatting.value = true;
  formatStatus.value = "";
  try {
    const result = await invoke("format_disk_path", {
      request: {
        path: selectedPath.value,
        filesystem,
        volume_name: volumeName,
        confirmation: formatConfirmation.value,
      },
    });
    formatStatus.value = t(lang.value, "format_success", {
      diskIdentifier: result.disk_identifier,
      volumeName: result.volume_name,
    });
    formatConfirmation.value = "";
    scanResult.value = null;
    await loadDisks();
  } catch (error) {
    formatStatus.value = t(lang.value, "format_failed", { error });
  } finally {
    isFormatting.value = false;
  }
}

async function exportScanReport() {
  if (!scanResult.value) {
    window.alert(t(lang.value, "scan_first"));
    return;
  }

  const path = await save({
    defaultPath: "mini-testdisk-scan-report.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;

  exportStatus.value = "";
  try {
    await invoke("export_scan_report_json", {
      path,
      result: scanResult.value,
    });
    exportStatus.value = t(lang.value, "export_success", { path });
  } catch (error) {
    exportStatus.value = t(lang.value, "export_failed", { error });
  }
}

onMounted(() => {
  loadDisks();
  loadFormatFilesystems();
});
</script>

<template>
  <header class="app-header">
    <div>
      <p class="eyebrow">{{ t(lang, "app_name") }}</p>
      <h1>{{ t(lang, "app_name") }}</h1>
      <p class="subtitle">{{ t(lang, "subtitle") }}</p>
    </div>
    <div class="header-actions">
      <div class="segmented" role="group" :aria-label="t(lang, 'theme_dark')">
        <button type="button" :class="{ active: theme === 'light' }" @click="setTheme('light')">
          {{ t(lang, "theme_light") }}
        </button>
        <button type="button" :class="{ active: theme === 'dark' }" @click="setTheme('dark')">
          {{ t(lang, "theme_dark") }}
        </button>
      </div>
      <div class="segmented" role="group" :aria-label="t(lang, 'lang_en')">
        <button type="button" :class="{ active: lang === 'zh' }" @click="setLanguage('zh')">
          {{ t(lang, "lang_zh") }}
        </button>
        <button type="button" :class="{ active: lang === 'en' }" @click="setLanguage('en')">
          {{ t(lang, "lang_en") }}
        </button>
      </div>
    </div>
  </header>

  <section class="toolbar">
    <label>
      {{ t(lang, "select_disk_label") }}
      <select v-model="selectedPath" :disabled="isLoadingDisks">
        <option value="">
          {{ isLoadingDisks ? t(lang, "loading") : diskOptions.length ? t(lang, "choose_disk") : t(lang, "no_disk") }}
        </option>
        <option v-for="disk in diskOptions" :key="disk.path" :value="disk.path">
          {{ disk.kind === "image" ? disk.name : disk.path }} ({{ formatSize(disk.size_bytes) }})
          {{ disk.kind === "image" ? "[镜像]" : disk.readable ? "" : "[需权限]" }}
        </option>
      </select>
    </label>
    <button type="button" @click="loadDisks">{{ t(lang, "refresh_disks") }}</button>
    <button type="button" @click="openImage">{{ t(lang, "open_image") }}</button>
    <button type="button" class="primary" :disabled="isScanning" @click="scanSelected">
      {{ isScanning ? t(lang, "analyzing") : t(lang, "analyze") }}
    </button>
  </section>

  <section v-if="selectedDisk" class="target-panel">
    <div class="section-head">
      <h2>{{ t(lang, "target_title") }}</h2>
      <span class="section-subtitle">{{ t(lang, "selected_disk_prefix") }}</span>
    </div>
    <div class="target-grid">
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
  </section>

  <section class="format-panel">
    <div class="section-head">
      <h2>{{ t(lang, "format_title") }}</h2>
      <span class="danger-chip">{{ t(lang, "format_title") }}</span>
    </div>
    <div class="format-grid">
      <label>
        {{ t(lang, "filesystem") }}
        <select v-model="selectedFormatFilesystem">
          <option
            v-for="filesystem in localizedFormatFilesystems"
            :key="filesystem.value"
            :value="filesystem.value"
          >
            {{ filesystem.label }}
          </option>
        </select>
      </label>
      <label>
        {{ t(lang, "volume_name") }}
        <input v-model="formatVolumeName" type="text" maxlength="32" placeholder="UNTITLED" />
      </label>
      <label>
        {{ t(lang, "confirm_target") }}
        <input
          v-model="formatConfirmation"
          type="text"
          :placeholder="formatConfirmationTarget || t(lang, 'choose_disk')"
        />
      </label>
      <button
        type="button"
        class="danger"
        :disabled="!canFormat || isFormatting"
        @click="formatSelectedDisk"
      >
        {{ isFormatting ? t(lang, "formatting") : t(lang, "format_disk") }}
      </button>
    </div>
    <p class="hint">
      {{ t(lang, "format_hint") }}
      <strong>{{ formatConfirmationTarget || "-" }}</strong>
    </p>
    <p v-if="formatStatus" class="format-status">{{ formatStatus }}</p>
  </section>

  <section v-if="loadError" class="warnings">{{ loadError }}</section>
  <section v-else-if="showMacDiskPermissionNotice" class="warnings">
    <strong>{{ t(lang, "permission_title") }}</strong>
    {{ t(lang, "permission_body") }}
  </section>

  <section v-if="scanResult" class="info-panel">
    <strong>{{ scanResult.disk_path }}</strong>
    &nbsp;·&nbsp; {{ t(lang, "size") }}: {{ formatSize(scanResult.disk_size) }}
    &nbsp;·&nbsp; {{ t(lang, "partition_table") }}: {{ scanResult.partitions.length }}
    &nbsp;·&nbsp; {{ t(lang, "lost_partitions") }}: {{ scanResult.lost_partitions.length }}
    <button type="button" class="inline-action" @click="exportScanReport">
      {{ t(lang, "report_export") }}
    </button>
  </section>
  <section v-if="exportStatus" class="info-panel">{{ exportStatus }}</section>
  <section v-if="scanResult?.warnings?.length" class="warnings">
    <strong>{{ t(lang, "warnings") }}</strong>
    <div v-for="warning in scanResult.warnings" :key="warning">! {{ warning }}</div>
  </section>

  <main class="grid">
    <div class="panel">
      <div class="section-head">
        <h2>{{ t(lang, "partition_table") }}</h2>
        <div class="badge">{{ tableTypeLabel }}</div>
      </div>
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

    <div class="panel">
      <div class="section-head">
        <h2>{{ t(lang, "lost_partitions") }}</h2>
        <span class="section-subtitle">{{ t(lang, "lost_hint") }}</span>
      </div>
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
  </main>

  <footer>
    <p>
      {{ t(lang, "footer") }}
      <a href="https://github.com/cgsecurity/testdisk">cgsecurity/testdisk</a>
    </p>
  </footer>
</template>
