<script setup>
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

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

const readableDiskCount = computed(() => diskOptions.value.filter((disk) => disk.readable).length);
const hasUnreadableSystemDisks = computed(() =>
  diskOptions.value.some((disk) => !disk.readable && disk.source === "system")
);
const showMacDiskPermissionNotice = computed(() =>
  hasUnreadableSystemDisks.value && readableDiskCount.value === 0
);
const tableTypeLabel = computed(() => {
  const typeMap = { gpt: "EFI GPT", mbr: "Legacy MBR", unknown: "未知" };
  const type = scanResult.value?.partition_table_type;
  return type ? typeMap[type] || type : "-";
});
const selectedDisk = computed(() => diskOptions.value.find((disk) => disk.path === selectedPath.value));
const formatConfirmationTarget = computed(() => selectedDisk.value?.path || "");
const canFormat = computed(() =>
  Boolean(
    selectedDisk.value?.source === "system" &&
      selectedPath.value &&
      selectedFormatFilesystem.value &&
      formatVolumeName.value.trim()
  )
);

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
    loadError.value = `加载失败: ${error}`;
    diskOptions.value = [];
  } finally {
    isLoadingDisks.value = false;
  }
}

async function loadFormatFilesystems() {
  try {
    formatFilesystems.value = await invoke("get_format_filesystems");
  } catch (error) {
    formatStatus.value = `加载格式化选项失败: ${error}`;
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
      { ...info, readable: true, isImage: true },
    ];
    selectedPath.value = info.path;
  } catch (error) {
    window.alert(`打开镜像失败: ${error}`);
  }
}

async function scanSelected() {
  if (!selectedPath.value) {
    window.alert("请先选择磁盘或打开镜像文件");
    return;
  }

  isScanning.value = true;
  try {
    scanResult.value = await invoke("scan_disk_path", { path: selectedPath.value });
  } catch (error) {
    window.alert(`扫描失败: ${error}`);
  } finally {
    isScanning.value = false;
  }
}

async function formatSelectedDisk() {
  if (!canFormat.value) {
    window.alert("请先选择磁盘、文件系统并填写卷名");
    return;
  }
  if (formatConfirmation.value.trim() !== formatConfirmationTarget.value) {
    window.alert(`请完整输入 ${formatConfirmationTarget.value} 以确认格式化`);
    return;
  }

  const filesystem = selectedFormatFilesystem.value;
  const volumeName = formatVolumeName.value.trim();
  const confirmed = window.confirm(
    `将格式化 ${selectedPath.value} 为 ${filesystem}，卷名 ${volumeName}。此操作会删除目标磁盘全部数据，是否继续？`
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
    formatStatus.value = `格式化完成: ${result.disk_identifier} -> ${result.volume_name}`;
    formatConfirmation.value = "";
    scanResult.value = null;
    await loadDisks();
  } catch (error) {
    formatStatus.value = `格式化失败: ${error}`;
  } finally {
    isFormatting.value = false;
  }
}

onMounted(() => {
  loadDisks();
  loadFormatFilesystems();
});
</script>

<template>
  <header>
    <h1>Mini TestDisk</h1>
    <p class="subtitle">Rust + Tauri 可行性验证 - 分区表分析与文件系统检测</p>
  </header>

  <section class="toolbar">
    <label>
      选择磁盘/镜像
      <select v-model="selectedPath" :disabled="isLoadingDisks">
        <option value="">
          {{ isLoadingDisks ? "-- 加载中 --" : diskOptions.length ? "-- 选择磁盘 --" : "未找到可读磁盘" }}
        </option>
        <option
          v-for="disk in diskOptions"
          :key="disk.path"
          :value="disk.path"
        >
          {{ disk.isImage ? disk.name : disk.path }} ({{ formatSize(disk.size_bytes) }})
          {{ disk.isImage ? "[镜像]" : disk.readable ? "" : "[需权限]" }}
        </option>
      </select>
    </label>
    <button type="button" @click="loadDisks">刷新磁盘列表</button>
    <button type="button" @click="openImage">打开磁盘镜像...</button>
    <button type="button" class="primary" :disabled="isScanning" @click="scanSelected">
      {{ isScanning ? "分析中..." : "分析分区" }}
    </button>
  </section>

  <section class="format-panel">
    <h2>格式化磁盘</h2>
    <div class="format-grid">
      <label>
        文件系统
        <select v-model="selectedFormatFilesystem">
          <option
            v-for="filesystem in formatFilesystems"
            :key="filesystem.value"
            :value="filesystem.value"
          >
            {{ filesystem.label }}
          </option>
        </select>
      </label>
      <label>
        卷名
        <input v-model="formatVolumeName" type="text" maxlength="32" placeholder="UNTITLED" />
      </label>
      <label>
        确认目标磁盘
        <input
          v-model="formatConfirmation"
          type="text"
          :placeholder="formatConfirmationTarget || '先选择磁盘'"
        />
      </label>
      <button
        type="button"
        class="danger"
        :disabled="!canFormat || isFormatting"
        @click="formatSelectedDisk"
      >
        {{ isFormatting ? "格式化中..." : "格式化磁盘" }}
      </button>
    </div>
    <p class="hint">
      仅用于 macOS 外置/USB 整盘格式化。请输入当前选择的目标磁盘路径
      <strong>{{ formatConfirmationTarget || "-" }}</strong>
      后才可执行。
    </p>
    <p v-if="formatStatus" class="format-status">{{ formatStatus }}</p>
  </section>

  <section v-if="loadError" class="warnings">{{ loadError }}</section>
  <section v-else-if="showMacDiskPermissionNotice" class="warnings">
    <strong>无法读取物理磁盘。</strong>
    macOS 不会为 /dev/disk* 原始块设备弹出普通应用授权窗口；这些设备通常需要以管理员权限运行。
    开发调试可使用 <code>sudo make dev</code>，或改用“打开磁盘镜像...”分析 .img/.iso/.raw 文件。
  </section>

  <section v-if="scanResult" class="info-panel">
    <strong>{{ scanResult.disk_path }}</strong>
    &nbsp;·&nbsp; 大小: {{ formatSize(scanResult.disk_size) }}
    &nbsp;·&nbsp; 分区: {{ scanResult.partitions.length }}
    &nbsp;·&nbsp; 丢失分区: {{ scanResult.lost_partitions.length }}
  </section>
  <section v-if="scanResult?.warnings?.length" class="warnings">
    <div v-for="warning in scanResult.warnings" :key="warning">! {{ warning }}</div>
  </section>

  <main class="grid">
    <div class="panel">
      <h2>分区表</h2>
      <div class="badge">{{ tableTypeLabel }}</div>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>名称</th>
            <th>起始 LBA</th>
            <th>大小</th>
            <th>类型</th>
            <th>文件系统</th>
            <th>状态</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!scanResult?.partitions?.length" class="empty-row">
            <td colspan="7">无数据</td>
          </tr>
          <tr v-for="partition in scanResult?.partitions" :key="partition.index">
            <td>{{ partition.index }}</td>
            <td>{{ partition.name }}</td>
            <td>{{ partition.start_lba }}</td>
            <td>{{ formatSize(partition.size_bytes) }}</td>
            <td>{{ partition.type_name }}</td>
            <td><span v-if="partition.filesystem" class="fs-tag">{{ fsLabel(partition) }}</span><span v-else>-</span></td>
            <td :class="statusClass(partition.status)">{{ partition.status }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="panel">
      <h2>丢失/删除分区 (深度扫描)</h2>
      <p class="hint">在 1MB 边界搜索文件系统签名，类似 TestDisk Deeper Search</p>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>名称</th>
            <th>起始 LBA</th>
            <th>大小</th>
            <th>文件系统</th>
            <th>来源</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!scanResult?.lost_partitions?.length" class="empty-row">
            <td colspan="6">未发现丢失分区</td>
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
      基于
      <a href="https://github.com/cgsecurity/testdisk">cgsecurity/testdisk</a>
      核心思路的简化实现
    </p>
  </footer>
</template>
