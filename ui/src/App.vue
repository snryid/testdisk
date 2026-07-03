<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { invoke, openDialog, saveDialog } from "./tauri-api";
import { conflictStatusLabel, filesystemLabel, partitionSourceLabel, partitionTableTypeLabel, t } from "./i18n";
import DiskSidebar from "./components/DiskSidebar.vue";
import AnalyzePanel from "./components/AnalyzePanel.vue";
import FormatPanel from "./components/FormatPanel.vue";
import ReportsPanel from "./components/ReportsPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";

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
const activeTab = ref("analyze");

const theme = ref(loadPreference(THEME_STORAGE_KEY, prefersDarkTheme() ? "dark" : "light"));
const lang = ref(loadPreference(LANG_STORAGE_KEY, "zh"));

const readableDiskCount = computed(() => diskOptions.value.filter((disk) => disk.readable).length);
const unreadableDiskCount = computed(() => diskOptions.value.filter((disk) => !disk.readable).length);
const hasAnyDisks = computed(() => diskOptions.value.length > 0);
const listStateNotice = computed(() => {
  if (isLoadingDisks.value || loadError.value) {
    return null;
  }

  if (!hasAnyDisks.value) {
    return {
      variant: "info",
      title: t(lang.value, "empty_list_title"),
      body: t(lang.value, "empty_list_body"),
    };
  }

  if (readableDiskCount.value === 0 && unreadableDiskCount.value > 0) {
    return {
      variant: "warning",
      title: t(lang.value, "no_readable_disk_title"),
      body: t(lang.value, "no_readable_disk_body"),
    };
  }

  return null;
});
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
const selectedDiskFormatTarget = computed(() => selectedDisk.value?.platform_id || "");
const selectedDiskStateNotice = computed(() => {
  const disk = selectedDisk.value;
  if (!disk) return null;

  if (disk.kind === "image" || disk.source === "image_file") {
    return {
      variant: "info",
      title: t(lang.value, "image_target_title"),
      body: t(lang.value, "image_target_body"),
    };
  }

  if (disk.access === "read_only") {
    return {
      variant: "info",
      title: t(lang.value, "read_only_target_title"),
      body: t(lang.value, "read_only_target_body"),
    };
  }

  if (!disk.readable || disk.access === "requires_elevation" || disk.access === "unavailable") {
    return {
      variant: "warning",
      title: t(lang.value, "unreadable_target_title"),
      body: t(lang.value, "unreadable_target_body"),
    };
  }

  return null;
});
const formatConfirmationTarget = computed(() => selectedDiskFormatTarget.value);
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
      selectedDisk.value?.kind === "physical" &&
      selectedDisk.value?.readable &&
      selectedDisk.value?.writable &&
      selectedDisk.value?.access === "read_write" &&
      selectedPath.value &&
      selectedFormatFilesystem.value &&
      formatVolumeName.value.trim()
  )
);
const formatBlockReason = computed(() => {
  const disk = selectedDisk.value;
  if (!disk) {
    return t(lang.value, "format_blocked_no_target");
  }
  if (disk.kind === "image" || disk.source === "image_file") {
    return t(lang.value, "format_blocked_image");
  }
  if (disk.access === "read_only") {
    return t(lang.value, "format_blocked_unreadable");
  }
  if (!disk.readable || !disk.writable || disk.access !== "read_write") {
    return t(lang.value, "format_blocked_unreadable");
  }
  return "";
});
const tabs = computed(() => [
  { key: "analyze", label: t(lang.value, "tab_analyze") },
  { key: "format", label: t(lang.value, "tab_format") },
  { key: "reports", label: t(lang.value, "tab_reports") },
  { key: "settings", label: t(lang.value, "tab_settings") },
]);

watch(theme, applyTheme, { immediate: true });
watch(lang, applyLanguage, { immediate: true });
watch(selectedPath, () => {
  loadFormatFilesystems();
});

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

function formatConfidence(value) {
  if (typeof value !== "number") return "-";
  return `${Math.round(value * 100)}%`;
}

function fsLabel(partition) {
  return partition.filesystem?.fs_type || "-";
}

function diskLabel(disk) {
  return disk.display_path || disk.name || disk.path || "-";
}

function diskOptionLabel(disk) {
  const suffixes = [];
  if (disk.kind === "image" || disk.source === "image_file") {
    suffixes.push(t(lang.value, "source_image"));
  } else {
    suffixes.push(t(lang.value, "source_system"));
  }
  if (disk.access) {
    suffixes.push(t(lang.value, `access_${disk.access}`));
  } else if (!disk.readable) {
    suffixes.push(t(lang.value, "access_requires_elevation"));
  }
  if (disk.safety) {
    suffixes.push(t(lang.value, `safety_${disk.safety}`));
  }
  return `${diskLabel(disk)} · ${formatSize(disk.size_bytes)} · ${suffixes.join(" · ")}`;
}

function statusClass(status) {
  if (status === "deleted") return "status-deleted";
  if (status === "bootable") return "status-bootable";
  return "";
}

function partitionSourceLabelFor(source) {
  return partitionSourceLabel(lang.value, source || "unknown");
}

function conflictStatusLabelFor(status) {
  return conflictStatusLabel(lang.value, status || "unknown");
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
    const available = await invoke("get_format_filesystems", {
      target: selectedDisk.value || null,
    });
    if (available.length > 0) {
      formatFilesystems.value = available;
    } else {
      formatFilesystems.value = await invoke("get_format_filesystems", {
        target: null,
      });
    }
    if (!available.some((filesystem) => filesystem.value === selectedFormatFilesystem.value)) {
      selectedFormatFilesystem.value = formatFilesystems.value[0]?.value || "";
    }
  } catch (error) {
    formatStatus.value = t(lang.value, "loading_format_options", { error });
  }
}

async function openImage() {
  const selected = await openDialog({
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
    activeTab.value = "analyze";
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
        target: selectedDisk.value,
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
    activeTab.value = "reports";
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

  const path = await saveDialog({
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
  <div class="app-shell">
    <header class="app-header">
      <div class="app-brand">
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

    <div class="workspace">
      <DiskSidebar
        :disk-options="diskOptions"
        :selected-path="selectedPath"
        :selected-disk="selectedDisk"
        :selected-disk-state-notice="selectedDiskStateNotice"
        :selected-disk-source-label="selectedDiskSourceLabel"
        :selected-disk-access-label="selectedDiskAccessLabel"
        :selected-disk-safety-label="selectedDiskSafetyLabel"
        :is-loading-disks="isLoadingDisks"
        :load-error="loadError"
        :list-state-notice="listStateNotice"
        :readable-disk-count="readableDiskCount"
        :has-any-disks="hasAnyDisks"
        :lang="lang"
        :format-size="formatSize"
        :disk-option-label="diskOptionLabel"
        @update:selected-path="selectedPath = $event"
        @refresh="loadDisks"
        @open-image="openImage"
      />

      <main class="workspace-main">
        <nav class="tab-strip" role="tablist" :aria-label="t(lang, 'app_name')">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            type="button"
            class="tab-button"
            :class="{ active: activeTab === tab.key }"
            @click="activeTab = tab.key"
          >
            {{ tab.label }}
          </button>
        </nav>

        <div class="panel-stage">
          <AnalyzePanel
            v-if="activeTab === 'analyze'"
            :lang="lang"
            :scan-result="scanResult"
            :table-type-label="tableTypeLabel"
            :is-scanning="isScanning"
            :format-size="formatSize"
            :format-confidence="formatConfidence"
            :fs-label="fsLabel"
            :partition-source-label="partitionSourceLabelFor"
            :conflict-status-label="conflictStatusLabelFor"
            :status-class="statusClass"
            @scan="scanSelected"
          />

          <FormatPanel
            v-else-if="activeTab === 'format'"
            :lang="lang"
            :selected-disk="selectedDisk"
            :localized-format-filesystems="localizedFormatFilesystems"
            :selected-format-filesystem="selectedFormatFilesystem"
            :format-volume-name="formatVolumeName"
            :format-confirmation="formatConfirmation"
            :format-confirmation-target="formatConfirmationTarget"
            :format-status="formatStatus"
            :format-block-reason="formatBlockReason"
            :can-format="canFormat"
            :is-formatting="isFormatting"
            @update:selected-format-filesystem="selectedFormatFilesystem = $event"
            @update:format-volume-name="formatVolumeName = $event"
            @update:format-confirmation="formatConfirmation = $event"
            @format="formatSelectedDisk"
          />

          <ReportsPanel
            v-else-if="activeTab === 'reports'"
            :lang="lang"
            :scan-result="scanResult"
            :export-status="exportStatus"
            :format-size="formatSize"
            @export-report="exportScanReport"
          />

          <SettingsPanel
            v-else
            :lang="lang"
            :theme="theme"
            @set-theme="setTheme"
            @set-language="setLanguage"
          />
        </div>
      </main>
    </div>
  </div>
</template>
