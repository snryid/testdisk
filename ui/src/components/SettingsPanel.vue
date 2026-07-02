<script setup>
import { t } from "../i18n";

defineProps({
  lang: { type: String, required: true },
  theme: { type: String, required: true },
});

const emit = defineEmits(["set-theme", "set-language"]);
</script>

<template>
  <section class="tab-panel">
    <div class="panel-toolbar">
      <div>
        <h2>{{ t(lang, "settings_panel_title") }}</h2>
        <p class="panel-subtitle">{{ t(lang, "subtitle") }}</p>
      </div>
    </div>

    <div class="settings-grid">
      <div class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "theme_dark") }}</h3>
        </div>
        <div class="segmented">
          <button type="button" :class="{ active: theme === 'light' }" @click="emit('set-theme', 'light')">
            {{ t(lang, "theme_light") }}
          </button>
          <button type="button" :class="{ active: theme === 'dark' }" @click="emit('set-theme', 'dark')">
            {{ t(lang, "theme_dark") }}
          </button>
        </div>
      </div>

      <div class="data-card">
        <div class="section-head">
          <h3>{{ t(lang, "lang_en") }}</h3>
        </div>
        <div class="segmented">
          <button type="button" :class="{ active: lang === 'zh' }" @click="emit('set-language', 'zh')">
            {{ t(lang, "lang_zh") }}
          </button>
          <button type="button" :class="{ active: lang === 'en' }" @click="emit('set-language', 'en')">
            {{ t(lang, "lang_en") }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
