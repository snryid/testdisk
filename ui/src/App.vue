<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { invoke, openDialog, saveDialog, tauriMajorVersion } from "./tauri-api";
import {
  conflictStatusLabel,
  filesystemLabel,
  partitionSourceLabel,
  partitionTableTypeLabel,
  t,
} from "./i18n";
import DiskSidebar from "./components/DiskSidebar.vue";
import OverviewPanel from "./components/OverviewPanel.vue";
import AnalyzePanel from "./components/AnalyzePanel.vue";
import FormatPanel from "./components/FormatPanel.vue";
import RepairPanel from "./components/RepairPanel.vue";
import RecoveryPanel from "./components/RecoveryPanel.vue";
import CarvingPanel from "./components/CarvingPanel.vue";
import ImagingPanel from "./components/ImagingPanel.vue";
import AutomationPanel from "./components/AutomationPanel.vue";
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
const isPreviewingFormat = ref(false);
const isDiagnosing = ref(false);
const isPreviewingRepair = ref(false);
const isRecovering = ref(false);
const isCarving = ref(false);
const isImaging = ref(false);
const loadError = ref("");
const selectedFormatFilesystem = ref("exfat");
const formatVolumeName = ref("UNTITLED");
const formatConfirmation = ref("");
const formatStatus = ref("");
const formatPreview = ref(null);
const repairTargetPath = ref("");
const diagnosisStatus = ref("");
const bootDiagnoses = ref([]);
const bootRepairPlans = ref([]);
const recoverySourcePath = ref("");
const recoveryDestinationDir = ref("");
const recoveryStatus = ref("");
const recoveryTranscript = ref(null);
const carvingSourcePath = ref("");
const carvingOutputDir = ref("");
const selectedCarvingFamilies = ref(["jpeg", "png", "pdf"]);
const carvingStatus = ref("");
const carvingTranscript = ref(null);
const imagingSourcePath = ref("");
const imagingOutputPath = ref("");
const imagingStatus = ref("");
const imagingTranscript = ref(null);
const exportStatus = ref("");
const automationStatus = ref("");
const activeTab = ref("overview");

const theme = ref(loadPreference(THEME_STORAGE_KEY, prefersDarkTheme() ? "dark" : "light"));
const lang = ref(loadPreference(LANG_STORAGE_KEY, "zh"));

const runtimeLabel = computed(() => `Tauri v${tauriMajorVersion()}`);
const readableDiskCount = computed(() => diskOptions.value.filter((disk) => disk.readable).length);
const unreadableDiskCount = computed(() => diskOptions.value.filter((disk) => !disk.readable).length);
const hasAnyDisks = computed(() => diskOptions.value.length > 0);
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
const formatConfirmationTarget = computed(() => selectedDiskFormatTarget.value);
const formatPreviewAllowed = computed(
  () => Boolean(canFormat.value && selectedFormatFilesystem.value && formatVolumeName.value.trim())
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
const workflowTabs = computed(() => [
  { key: "overview", label: t(lang.value, "tab_overview") },
  { key: "analyze", label: t(lang.value, "tab_analyze") },
  { key: "format", label: t(lang.value, "tab_format") },
  { key: "repair", label: t(lang.value, "tab_repair") },
  { key: "recovery", label: t(lang.value, "tab_recovery") },
  { key: "carving", label: t(lang.value, "tab_carving") },
  { key: "imaging", label: t(lang.value, "tab_imaging") },
  { key: "automation", label: t(lang.value, "tab_automation") },
  { key: "reports", label: t(lang.value, "tab_reports") },
  { key: "settings", label: t(lang.value, "tab_settings") },
]);
const commandTemplates = computed(() => {
  const input = selectedDisk.value?.path || "<disk-or-image>";
  const target = selectedDisk.value?.platform_id || "<target-id>";
  const filesystem = selectedFormatFilesystem.value || "<filesystem>";
  const volumeName = formatVolumeName.value.trim() || "UNTITLED";
  const quotedInput = quoteCommandArg(input);
  const quotedTarget = quoteCommandArg(target);
  const quotedFilesystem = quoteCommandArg(filesystem);
  const quotedVolume = quoteCommandArg(volumeName);

  return [
    {
      key: "scan",
      title: t(lang.value, "automation_scan_title"),
      description: t(lang.value, "automation_scan_desc"),
      command: `mini-testdisk scan --input ${quotedInput} --output report.json`,
    },
    {
      key: "validate-plan",
      title: t(lang.value, "automation_validate_title"),
      description: t(lang.value, "automation_validate_desc"),
      command: "mini-testdisk validate-plan --plan plan.json",
    },
    {
      key: "list-filesystems",
      title: t(lang.value, "automation_filesystems_title"),
      description: t(lang.value, "automation_filesystems_desc"),
      command: "mini-testdisk list-filesystems",
    },
    {
      key: "format",
      title: t(lang.value, "automation_format_title"),
      description: t(lang.value, "automation_format_desc"),
      command: `mini-testdisk format --target ${quotedTarget} --filesystem ${quotedFilesystem} --name ${quotedVolume} --confirm ${quotedTarget} --unsafe`,
    },
  ];
});

watch(theme, applyTheme, { immediate: true });
watch(lang, applyLanguage, { immediate: true });
watch(selectedPath, async () => {
  await loadFormatFilesystems();
  ensureWorkflowDefaults();
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

function quoteCommandArg(value) {
  if (!value || value.includes("<")) return value;
  return `"${value.replaceAll('"', '\\"')}"`;
}

function ensureWorkflowDefaults() {
  const diskPath = selectedDisk.value?.path || "";
  if (!diskPath) return;
  if (!repairTargetPath.value) repairTargetPath.value = diskPath;
  if (!recoverySourcePath.value) recoverySourcePath.value = diskPath;
  if (!carvingSourcePath.value) carvingSourcePath.value = diskPath;
  if (!imagingSourcePath.value) imagingSourcePath.value = diskPath;
}

function buildFormatRequest(confirmationOverride = null) {
  return {
    path: selectedPath.value,
    target: selectedDisk.value,
    filesystem: selectedFormatFilesystem.value,
    volume_name: formatVolumeName.value.trim(),
    confirmation: confirmationOverride ?? formatConfirmation.value,
  };
}

function setWorkflowTab(tab) {
  activeTab.value = tab;
}

async function loadDisks() {
  isLoadingDisks.value = true;
  loadError.value = "";
  try {
    diskOptions.value = await invoke("get_disks");
    ensureWorkflowDefaults();
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
    diskOptions.value = [...diskOptions.value.filter((disk) => disk.path !== info.path), info];
    selectedPath.value = info.path;
    ensureWorkflowDefaults();
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
    setWorkflowTab("analyze");
  } catch (error) {
    window.alert(t(lang.value, "scan_failed", { error }));
  } finally {
    isScanning.value = false;
  }
}

async function previewFormatPlan() {
  const request = buildFormatRequest(selectedDisk.value?.platform_id || selectedPath.value);
  if (!request.path || !request.target || !request.filesystem || !request.volume_name) {
    window.alert(t(lang.value, "choose_format_first"));
    return;
  }

  isPreviewingFormat.value = true;
  formatStatus.value = "";
  try {
    formatPreview.value = await invoke("preview_format_plan_command", {
      request,
    });
    formatStatus.value = t(lang.value, "format_preview_ready");
  } catch (error) {
    formatStatus.value = t(lang.value, "format_failed", { error });
  } finally {
    isPreviewingFormat.value = false;
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
      request: buildFormatRequest(),
    });
    formatStatus.value = t(lang.value, "format_success", {
      diskIdentifier: result.disk_identifier,
      volumeName: result.volume_name,
    });
    formatConfirmation.value = "";
    scanResult.value = null;
    setWorkflowTab("reports");
    await loadDisks();
  } catch (error) {
    formatStatus.value = t(lang.value, "format_failed", { error });
  } finally {
    isFormatting.value = false;
  }
}

async function diagnoseBootSector() {
  const path = repairTargetPath.value.trim() || selectedPath.value;
  if (!path) {
    window.alert(t(lang.value, "choose_target_first"));
    return;
  }

  isDiagnosing.value = true;
  diagnosisStatus.value = "";
  try {
    bootDiagnoses.value = await invoke("diagnose_boot_sectors_command", { path });
    diagnosisStatus.value = t(lang.value, "repair_diagnosis_ready");
  } catch (error) {
    diagnosisStatus.value = t(lang.value, "repair_failed", { error });
  } finally {
    isDiagnosing.value = false;
  }
}

async function previewBootRepair() {
  const path = repairTargetPath.value.trim() || selectedPath.value;
  if (!path) {
    window.alert(t(lang.value, "choose_target_first"));
    return;
  }

  isPreviewingRepair.value = true;
  diagnosisStatus.value = "";
  try {
    bootRepairPlans.value = await invoke("preview_boot_sector_repair_command", { path });
    diagnosisStatus.value = t(lang.value, "repair_preview_ready");
  } catch (error) {
    diagnosisStatus.value = t(lang.value, "repair_failed", { error });
  } finally {
    isPreviewingRepair.value = false;
  }
}

async function chooseRecoveryDestination() {
  const selected = await openDialog({ directory: true, multiple: false });
  if (selected) {
    recoveryDestinationDir.value = selected;
  }
}

async function recoverDeletedFiles() {
  const sourcePath = recoverySourcePath.value.trim() || selectedPath.value;
  if (!sourcePath || !recoveryDestinationDir.value.trim()) {
    window.alert(t(lang.value, "choose_recovery_first"));
    return;
  }

  isRecovering.value = true;
  recoveryStatus.value = "";
  try {
    recoveryTranscript.value = await invoke("recover_deleted_files", {
      source_path: sourcePath,
      destination_dir: recoveryDestinationDir.value.trim(),
    });
    recoveryStatus.value = t(lang.value, "recovery_complete");
  } catch (error) {
    recoveryStatus.value = t(lang.value, "recovery_failed", { error });
  } finally {
    isRecovering.value = false;
  }
}

async function chooseCarvingOutput() {
  const selected = await openDialog({ directory: true, multiple: false });
  if (selected) {
    carvingOutputDir.value = selected;
  }
}

async function carveFiles() {
  const sourcePath = carvingSourcePath.value.trim() || selectedPath.value;
  if (!sourcePath || !carvingOutputDir.value.trim() || selectedCarvingFamilies.value.length === 0) {
    window.alert(t(lang.value, "choose_carving_first"));
    return;
  }

  isCarving.value = true;
  carvingStatus.value = "";
  try {
    carvingTranscript.value = await invoke("carve_files_command", {
      source_path: sourcePath,
      output_dir: carvingOutputDir.value.trim(),
      families: selectedCarvingFamilies.value,
    });
    carvingStatus.value = t(lang.value, "carving_complete");
  } catch (error) {
    carvingStatus.value = t(lang.value, "carving_failed", { error });
  } finally {
    isCarving.value = false;
  }
}

async function chooseImagingOutput() {
  const selected = await saveDialog({
    defaultPath: "disk-image.raw",
    filters: [{ name: "Raw Image", extensions: ["raw", "img", "dd"] }],
  });
  if (selected) {
    imagingOutputPath.value = selected;
  }
}

async function imageDisk() {
  const sourcePath = imagingSourcePath.value.trim() || selectedPath.value;
  if (!sourcePath || !imagingOutputPath.value.trim()) {
    window.alert(t(lang.value, "choose_imaging_first"));
    return;
  }

  isImaging.value = true;
  imagingStatus.value = "";
  try {
    imagingTranscript.value = await invoke("image_source_command", {
      source_path: sourcePath,
      output_path: imagingOutputPath.value.trim(),
    });
    imagingStatus.value = t(lang.value, "imaging_complete");
  } catch (error) {
    imagingStatus.value = t(lang.value, "imaging_failed", { error });
  } finally {
    isImaging.value = false;
  }
}

async function copyCommand(command) {
  try {
    await navigator.clipboard.writeText(command);
    automationStatus.value = t(lang.value, "command_copied");
  } catch {
    automationStatus.value = command;
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
  ensureWorkflowDefaults();
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
        <span class="badge">{{ runtimeLabel }}</span>
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
            v-for="tab in workflowTabs"
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
          <OverviewPanel
            v-if="activeTab === 'overview'"
            :lang="lang"
            :runtime-label="runtimeLabel"
            :selected-disk="selectedDisk"
            :selected-disk-source-label="selectedDiskSourceLabel"
            :selected-disk-access-label="selectedDiskAccessLabel"
            :selected-disk-safety-label="selectedDiskSafetyLabel"
            :scan-result="scanResult"
            :format-size="formatSize"
            :format-status="formatStatus"
            :repair-status="diagnosisStatus"
            :recovery-status="recoveryStatus"
            :imaging-status="imagingStatus"
            :carving-status="carvingStatus"
            :automation-status="automationStatus"
            @go-tab="setWorkflowTab"
          />

          <AnalyzePanel
            v-else-if="activeTab === 'analyze'"
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
            :format-preview="formatPreview"
            :format-block-reason="formatBlockReason"
            :can-format="canFormat"
            :can-preview-format="formatPreviewAllowed"
            :is-previewing-format="isPreviewingFormat"
            :is-formatting="isFormatting"
            @update:selected-format-filesystem="selectedFormatFilesystem = $event"
            @update:format-volume-name="formatVolumeName = $event"
            @update:format-confirmation="formatConfirmation = $event"
            @preview-format="previewFormatPlan"
            @format="formatSelectedDisk"
          />

          <RepairPanel
            v-else-if="activeTab === 'repair'"
            :lang="lang"
            :selected-disk="selectedDisk"
            :target-path="repairTargetPath"
            :diagnosis-status="diagnosisStatus"
            :diagnoses="bootDiagnoses"
            :repair-plans="bootRepairPlans"
            :is-diagnosing="isDiagnosing"
            :is-previewing="isPreviewingRepair"
            @update:target-path="repairTargetPath = $event"
            @use-selected-target="repairTargetPath = selectedDisk?.path || ''"
            @diagnose="diagnoseBootSector"
            @preview-repair="previewBootRepair"
          />

          <RecoveryPanel
            v-else-if="activeTab === 'recovery'"
            :lang="lang"
            :selected-disk="selectedDisk"
            :source-path="recoverySourcePath"
            :destination-dir="recoveryDestinationDir"
            :recovery-status="recoveryStatus"
            :recovery-transcript="recoveryTranscript"
            :is-recovering="isRecovering"
            @update:source-path="recoverySourcePath = $event"
            @update:destination-dir="recoveryDestinationDir = $event"
            @use-selected-target="recoverySourcePath = selectedDisk?.path || ''"
            @choose-destination="chooseRecoveryDestination"
            @recover="recoverDeletedFiles"
          />

          <CarvingPanel
            v-else-if="activeTab === 'carving'"
            :lang="lang"
            :selected-disk="selectedDisk"
            :source-path="carvingSourcePath"
            :output-dir="carvingOutputDir"
            :selected-families="selectedCarvingFamilies"
            :carving-status="carvingStatus"
            :carving-transcript="carvingTranscript"
            :is-carving="isCarving"
            @update:source-path="carvingSourcePath = $event"
            @update:output-dir="carvingOutputDir = $event"
            @update:selected-families="selectedCarvingFamilies = $event"
            @use-selected-target="carvingSourcePath = selectedDisk?.path || ''"
            @choose-output="chooseCarvingOutput"
            @carve="carveFiles"
          />

          <ImagingPanel
            v-else-if="activeTab === 'imaging'"
            :lang="lang"
            :selected-disk="selectedDisk"
            :source-path="imagingSourcePath"
            :output-path="imagingOutputPath"
            :imaging-status="imagingStatus"
            :imaging-transcript="imagingTranscript"
            :is-imaging="isImaging"
            @update:source-path="imagingSourcePath = $event"
            @update:output-path="imagingOutputPath = $event"
            @use-selected-target="imagingSourcePath = selectedDisk?.path || ''"
            @choose-output="chooseImagingOutput"
            @image="imageDisk"
          />

          <AutomationPanel
            v-else-if="activeTab === 'automation'"
            :lang="lang"
            :runtime-label="runtimeLabel"
            :selected-disk="selectedDisk"
            :command-templates="commandTemplates"
            :automation-status="automationStatus"
            @copy-command="copyCommand"
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
            :runtime-label="runtimeLabel"
            @set-theme="setTheme"
            @set-language="setLanguage"
          />
        </div>
      </main>
    </div>
  </div>
</template>
