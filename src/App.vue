<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface InvalidArtifact {
  folder: string;
  base_name: string;
  reason: string;
}

interface CleanItem {
  folder: string;
  base_name: string;
}

interface CleanResult {
  deleted_count: number;
  errors: string[];
}

const repoPath = ref("");
const customPath = ref("");
const invalidArtifacts = ref<InvalidArtifact[]>([]);
const isScanning = ref(false);
const isCleaning = ref(false);
const errorMsg = ref("");
const successMsg = ref("");
const showSettings = ref(false);

onMounted(async () => {
  try {
    repoPath.value = await invoke<string>("get_maven_repo_path");
    customPath.value = repoPath.value;
    // 自动启动扫描
    await scanArtifacts();
  } catch (err) {
    errorMsg.value = `获取仓库路径失败: ${err}`;
  }
});

async function scanArtifacts() {
  errorMsg.value = "";
  successMsg.value = "";
  invalidArtifacts.value = [];

  if (!customPath.value.trim()) {
    errorMsg.value = "请输入仓库路径";
    return;
  }

  isScanning.value = true;
  showSettings.value = false; // 扫描开始后关闭设置弹窗

  try {
    const results = await invoke<InvalidArtifact[]>("scan_invalid_artifacts", {
      repoPath: customPath.value,
    });
    invalidArtifacts.value = results;

    if (results.length === 0) {
      successMsg.value = "恭喜！没有发现损坏文件。";
    } else {
      successMsg.value = `发现 ${results.length} 个损坏的构件。`;
    }
  } catch (err) {
    errorMsg.value = `扫描失败: ${err}`;
  } finally {
    isScanning.value = false;
  }
}

async function cleanAll() {
  if (invalidArtifacts.value.length === 0) {
    return;
  }

  if (!confirm(`确认删除全部 ${invalidArtifacts.value.length} 个损坏的构件吗？此操作不可撤销！`)) {
    return;
  }

  errorMsg.value = "";
  successMsg.value = "";
  isCleaning.value = true;

  try {
    const items: CleanItem[] = invalidArtifacts.value.map((artifact) => ({
      folder: artifact.folder,
      base_name: artifact.base_name,
    }));

    const result = await invoke<CleanResult>("clean_artifacts", { items });

    if (result.errors.length > 0) {
      errorMsg.value = `删除完成，但有 ${result.errors.length} 个错误:\n${result.errors.slice(0, 5).join("\n")}`;
    } else {
      successMsg.value = `成功删除 ${result.deleted_count} 个文件！`;
    }

    invalidArtifacts.value = [];
  } catch (err) {
    errorMsg.value = `清理失败: ${err}`;
  } finally {
    isCleaning.value = false;
  }
}
</script>

<template>
  <div class="h-screen bg-gradient-to-br from-gray-50 to-gray-100 flex flex-col">
    <div class="max-w-6xl w-full mx-auto flex flex-col h-full p-8">
      <!-- 标题和设置按钮 -->
      <div class="text-center mb-6 relative flex-shrink-0">
        <h1 class="text-4xl font-bold text-gray-800 mb-2">Maven 仓库清理工具</h1>
        <p class="text-gray-600">检测并清除损坏的 JAR/POM 文件</p>

        <!-- 设置按钮 -->
        <button
          @click="showSettings = true"
          class="absolute top-0 right-0 p-2 text-gray-600 hover:text-gray-800 hover:bg-gray-200 rounded-lg transition"
          title="设置"
        >
          <svg
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
          </svg>
        </button>
      </div>

      <!-- 设置弹窗 -->
      <div
        v-if="showSettings"
        class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
        @click.self="showSettings = false"
      >
        <div class="bg-white rounded-xl shadow-2xl p-6 max-w-2xl w-full mx-4">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-2xl font-bold text-gray-800">仓库设置</h2>
            <button
              @click="showSettings = false"
              class="text-gray-500 hover:text-gray-700 transition"
            >
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Maven 仓库路径
            </label>
            <input
              v-model="customPath"
              type="text"
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none transition"
              placeholder="请输入 Maven 仓库路径"
            />
            <p class="text-sm text-gray-500 mt-2">
              自动检测路径（优先级：mvn -v → $MAVEN_HOME/conf/settings.xml → ~/.m2/settings.xml → 默认路径）
            </p>
          </div>

          <div class="flex gap-3 justify-end">
            <button
              @click="showSettings = false"
              class="px-6 py-2 bg-gray-200 hover:bg-gray-300 text-gray-700 font-medium rounded-lg transition"
            >
              取消
            </button>
            <button
              @click="scanArtifacts"
              :disabled="isScanning"
              class="px-6 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-medium rounded-lg transition shadow-md hover:shadow-lg"
            >
              {{ isScanning ? "扫描中..." : "开始扫描" }}
            </button>
          </div>
        </div>
      </div>

      <!-- 消息提示 -->
      <div v-if="errorMsg" class="bg-red-50 border-l-4 border-red-500 text-red-700 p-4 mb-4 rounded-lg flex-shrink-0">
        <p class="font-medium">错误</p>
        <p class="text-sm whitespace-pre-line">{{ errorMsg }}</p>
      </div>

      <div v-if="successMsg" class="bg-green-50 border-l-4 border-green-500 text-green-700 p-4 mb-4 rounded-lg flex-shrink-0">
        <p class="text-sm">{{ successMsg }}</p>
      </div>

      <!-- 结果列表 -->
      <div v-if="invalidArtifacts.length > 0" class="bg-white rounded-xl shadow-lg p-6 flex flex-col flex-1 min-h-0">
        <div class="flex items-center justify-between mb-4 flex-shrink-0">
          <h2 class="text-xl font-semibold text-gray-800">
            损坏的构件 ({{ invalidArtifacts.length }} 项)
          </h2>
          <button
            @click="cleanAll"
            :disabled="isCleaning"
            class="px-6 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-400 text-white font-medium rounded-lg transition shadow-md hover:shadow-lg"
          >
            {{ isCleaning ? "清理中..." : `清除全部 (${invalidArtifacts.length})` }}
          </button>
        </div>

        <div class="flex-1 overflow-y-auto border border-gray-200 rounded-lg min-h-0">
          <table class="w-full text-sm">
            <thead class="bg-gray-50 sticky top-0">
              <tr>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase tracking-wider">
                  路径
                </th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase tracking-wider">
                  构件名称
                </th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase tracking-wider">
                  损坏原因
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200">
              <tr v-for="(artifact, index) in invalidArtifacts" :key="index" class="hover:bg-gray-50">
                <td class="px-4 py-3 text-gray-700 font-mono text-xs">
                  {{ artifact.folder }}
                </td>
                <td class="px-4 py-3 text-gray-900 font-medium">
                  {{ artifact.base_name }}
                </td>
                <td class="px-4 py-3">
                  <span class="px-2 py-1 bg-yellow-100 text-yellow-800 text-xs rounded-full">
                    {{ artifact.reason }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
