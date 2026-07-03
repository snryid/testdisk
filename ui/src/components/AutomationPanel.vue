<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  runtimeLabel: { type: String, required: true },
  selectedDisk: { type: Object, default: null },
  commandTemplates: { type: Array, required: true },
  automationStatus: { type: String, default: "" },
});

const emit = defineEmits(["copy-command"]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "tab_automation") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "automation_panel_hint") }}</p>
      </div>
      <span class="badge">{{ runtimeLabel }}</span>
    </div>

    <div class="workflow-grid automation-grid">
      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "automation_commands") }}</h3>
          <span class="section-subtitle">{{ automationStatus || t(lang, "automation_ready") }}</span>
        </div>
        <div class="stack-list">
          <article v-for="command in commandTemplates" :key="command.key" class="inner-card">
            <div class="section-head">
              <h4>{{ command.title }}</h4>
              <button type="button" class="inline-action" @click="emit('copy-command', command.command)">
                {{ t(lang, "copy_command") }}
              </button>
            </div>
            <p class="muted-block">{{ command.description }}</p>
            <pre class="code-block">{{ command.command }}</pre>
          </article>
        </div>
      </article>

      <article class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "automation_context") }}</h3>
          <span class="section-subtitle">{{ t(lang, "automation_context_hint") }}</span>
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
            <span>{{ t(lang, "access") }}</span>
            <strong>{{ selectedDisk.access || "-" }}</strong>
          </div>
        </div>
        <p v-else class="muted-block">{{ t(lang, "overview_no_target") }}</p>
      </article>
    </div>
  </section>
</template>
