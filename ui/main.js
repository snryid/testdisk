const { invoke } = window.__TAURI__.core;

async function openFileDialog() {
  const { open } = window.__TAURI__.dialog;
  return open({
    multiple: false,
    filters: [{ name: "Disk Image", extensions: ["img", "dmg", "iso", "raw", "bin"] }],
  });
}

const diskSelect = document.getElementById("disk-select");
const btnRefresh = document.getElementById("btn-refresh");
const btnOpenImage = document.getElementById("btn-open-image");
const btnScan = document.getElementById("btn-scan");
const diskInfo = document.getElementById("disk-info");
const warningsEl = document.getElementById("warnings");
const tableType = document.getElementById("table-type");
const partitionsBody = document.querySelector("#partitions-table tbody");
const lostBody = document.querySelector("#lost-table tbody");

function formatSize(bytes) {
  if (!bytes) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function renderDiskOptions(disks) {
  diskSelect.innerHTML = "";
  if (disks.length === 0) {
    diskSelect.innerHTML = '<option value="">未找到可读磁盘</option>';
    return;
  }
  diskSelect.innerHTML = '<option value="">-- 选择磁盘 --</option>';
  for (const d of disks) {
    const opt = document.createElement("option");
    opt.value = d.path;
    opt.textContent = `${d.path} (${formatSize(d.size_bytes)})${d.readable ? "" : " [需权限]"}`;
    opt.disabled = !d.readable;
    diskSelect.appendChild(opt);
  }
}

async function loadDisks() {
  try {
    const disks = await invoke("get_disks");
    renderDiskOptions(disks);
  } catch (e) {
    diskSelect.innerHTML = `<option value="">加载失败: ${e}</option>`;
  }
}

function renderPartitions(parts, tbody, cols) {
  tbody.innerHTML = "";
  if (!parts || parts.length === 0) {
    tbody.innerHTML = `<tr class="empty-row"><td colspan="${cols}">无数据</td></tr>`;
    return;
  }
  for (const p of parts) {
    const tr = document.createElement("tr");
    const fs = p.filesystem
      ? `<span class="fs-tag">${p.filesystem.fs_type}</span>`
      : "-";
    const statusClass =
      p.status === "deleted" ? "status-deleted" :
      p.status === "bootable" ? "status-bootable" : "";
    tr.innerHTML = `
      <td>${p.index}</td>
      <td>${escapeHtml(p.name)}</td>
      <td>${p.start_lba}</td>
      <td>${formatSize(p.size_bytes)}</td>
      <td>${escapeHtml(p.type_name)}</td>
      <td>${fs}</td>
      <td class="${statusClass}">${p.status}</td>
    `;
    tbody.appendChild(tr);
  }
}

function renderLost(parts) {
  lostBody.innerHTML = "";
  if (!parts || parts.length === 0) {
    lostBody.innerHTML = '<tr class="empty-row"><td colspan="6">未发现丢失分区</td></tr>';
    return;
  }
  for (const p of parts) {
    const tr = document.createElement("tr");
    const fs = p.filesystem ? p.filesystem.fs_type : "-";
    tr.innerHTML = `
      <td>${p.index}</td>
      <td>${escapeHtml(p.name)}</td>
      <td>${p.start_lba}</td>
      <td>${formatSize(p.size_bytes)}</td>
      <td><span class="fs-tag">${fs}</span></td>
      <td>${p.source}</td>
    `;
    lostBody.appendChild(tr);
  }
}

function escapeHtml(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function showResult(result) {
  diskInfo.classList.remove("hidden");
  diskInfo.innerHTML = `
    <strong>${escapeHtml(result.disk_path)}</strong>
    &nbsp;·&nbsp; 大小: ${formatSize(result.disk_size)}
    &nbsp;·&nbsp; 分区: ${result.partitions.length}
    &nbsp;·&nbsp; 丢失分区: ${result.lost_partitions.length}
  `;

  if (result.warnings?.length) {
    warningsEl.classList.remove("hidden");
    warningsEl.innerHTML = result.warnings.map((w) => `⚠ ${escapeHtml(w)}`).join("<br>");
  } else {
    warningsEl.classList.add("hidden");
  }

  const typeMap = { gpt: "EFI GPT", mbr: "Legacy MBR", unknown: "未知" };
  tableType.textContent = typeMap[result.partition_table_type] || result.partition_table_type;

  renderPartitions(result.partitions, partitionsBody, 7);
  renderLost(result.lost_partitions);
}

async function scanSelected() {
  const path = diskSelect.value;
  if (!path) {
    alert("请先选择磁盘或打开镜像文件");
    return;
  }
  btnScan.disabled = true;
  btnScan.textContent = "分析中...";
  try {
    const result = await invoke("scan_disk_path", { path });
    showResult(result);
  } catch (e) {
    alert(`扫描失败: ${e}`);
  } finally {
    btnScan.disabled = false;
    btnScan.textContent = "分析分区";
  }
}

btnRefresh.addEventListener("click", loadDisks);
btnScan.addEventListener("click", scanSelected);

btnOpenImage.addEventListener("click", async () => {
  const selected = await openFileDialog();
  if (!selected) return;

  try {
    const info = await invoke("open_image", { path: selected });
    const opt = document.createElement("option");
    opt.value = info.path;
    opt.textContent = `${info.name} (${formatSize(info.size_bytes)}) [镜像]`;
    opt.selected = true;
    diskSelect.appendChild(opt);
  } catch (e) {
    alert(`打开镜像失败: ${e}`);
  }
});

loadDisks();
