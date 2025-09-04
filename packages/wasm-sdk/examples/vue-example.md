# Vue.js Integration Example

This guide demonstrates how to integrate the Dash Platform WASM SDK into a Vue.js application using the modern WasmSDK wrapper with Vue's Composition API.

## Table of Contents

- [Installation](#installation)
- [Composable Implementation](#composable-implementation)
- [Plugin Pattern](#plugin-pattern)
- [Component Examples](#component-examples)
- [State Management with Pinia](#state-management-with-pinia)
- [Provide/Inject Pattern](#provideinject-pattern)
- [Complete Application Example](#complete-application-example)
- [TypeScript Support](#typescript-support)
- [SSR/Nuxt.js Integration](#ssrnuxtjs-integration)
- [Troubleshooting](#troubleshooting)

## Installation

```bash
# Vue 3 with Composition API
npm install @dashevo/dash-wasm-sdk vue@next

# For TypeScript projects
npm install --save-dev typescript @vue/tsconfig

# Optional: Pinia for state management
npm install pinia

# Optional: Nuxt.js for SSR
npm install nuxt@3
```

## Composable Implementation

### Basic useWasmSDK Composable

```javascript
// composables/useWasmSDK.js
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

export function useWasmSDK(config) {
    const sdk = ref(null);
    const loading = ref(false);
    const error = ref(null);
    const initialized = ref(false);

    let sdkInstance = null;

    const initialize = async () => {
        if (sdkInstance) return;

        try {
            loading.value = true;
            error.value = null;
            
            sdkInstance = new WasmSDK(config);
            await sdkInstance.initialize();
            
            sdk.value = sdkInstance;
            initialized.value = true;
        } catch (err) {
            error.value = err;
            console.error('SDK initialization failed:', err);
        } finally {
            loading.value = false;
        }
    };

    const destroy = async () => {
        if (sdkInstance) {
            await sdkInstance.destroy();
            sdkInstance = null;
            sdk.value = null;
            initialized.value = false;
        }
    };

    onMounted(initialize);
    onUnmounted(destroy);

    // Watch config changes and reinitialize if needed
    watch(() => JSON.stringify(config), async () => {
        await destroy();
        await initialize();
    });

    return {
        sdk: readonly(sdk),
        loading: readonly(loading),
        error: readonly(error),
        initialized: readonly(initialized),
        initialize,
        destroy
    };
}
```

### Advanced Composable with Reactive Features

```javascript
// composables/useWasmSDKAdvanced.js
import { ref, computed, onMounted, onUnmounted, watch, readonly } from 'vue';
import { WasmSDK, WasmTransportError } from '@dashevo/dash-wasm-sdk';

export function useWasmSDKAdvanced(config, options = {}) {
    const {
        maxRetries = 3,
        retryDelay = 1000,
        autoReconnect = true,
        enableResourceCleanup = true
    } = options;

    // Reactive state
    const sdk = ref(null);
    const loading = ref(false);
    const error = ref(null);
    const initialized = ref(false);
    const retryCount = ref(0);
    const connected = ref(false);

    let sdkInstance = null;
    let retryTimeout = null;
    let cleanupInterval = null;

    // Computed properties
    const isReady = computed(() => initialized.value && !loading.value && !error.value);
    const canRetry = computed(() => error.value && retryCount.value < maxRetries);
    const connectionStatus = computed(() => {
        if (loading.value) return 'connecting';
        if (error.value) return 'error';
        if (connected.value) return 'connected';
        return 'disconnected';
    });

    const initialize = async (attempt = 1) => {
        if (sdkInstance && initialized.value) return;

        try {
            loading.value = true;
            error.value = null;
            
            sdkInstance = new WasmSDK(config);
            await sdkInstance.initialize();
            
            sdk.value = sdkInstance;
            initialized.value = true;
            connected.value = true;
            retryCount.value = 0;

            // Set up resource cleanup if enabled
            if (enableResourceCleanup) {
                cleanupInterval = setInterval(() => {
                    if (sdkInstance) {
                        sdkInstance.cleanupResources();
                    }
                }, 300000); // 5 minutes
            }
        } catch (err) {
            error.value = err;
            connected.value = false;
            
            // Auto-retry for transport errors
            if (err instanceof WasmTransportError && attempt < maxRetries && autoReconnect) {
                retryCount.value = attempt;
                const delay = retryDelay * attempt;
                
                retryTimeout = setTimeout(() => {
                    initialize(attempt + 1);
                }, delay);
            }
        } finally {
            loading.value = false;
        }
    };

    const destroy = async () => {
        // Clear timers
        if (retryTimeout) {
            clearTimeout(retryTimeout);
            retryTimeout = null;
        }
        if (cleanupInterval) {
            clearInterval(cleanupInterval);
            cleanupInterval = null;
        }

        // Destroy SDK
        if (sdkInstance) {
            await sdkInstance.destroy();
            sdkInstance = null;
        }

        // Reset state
        sdk.value = null;
        initialized.value = false;
        connected.value = false;
        error.value = null;
        retryCount.value = 0;
    };

    const retry = () => {
        if (canRetry.value) {
            initialize(1);
        }
    };

    // Lifecycle hooks
    onMounted(initialize);
    onUnmounted(destroy);

    // Watch config changes
    watch(() => JSON.stringify(config), async () => {
        await destroy();
        await initialize();
    });

    return {
        // Reactive state (readonly)
        sdk: readonly(sdk),
        loading: readonly(loading),
        error: readonly(error),
        initialized: readonly(initialized),
        connected: readonly(connected),
        retryCount: readonly(retryCount),
        
        // Computed
        isReady,
        canRetry,
        connectionStatus,
        
        // Methods
        initialize,
        destroy,
        retry
    };
}
```

## Plugin Pattern

### SDK Plugin

```javascript
// plugins/wasmSDK.js
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

export default {
    install(app, options) {
        const config = {
            network: 'testnet',
            transport: {
                url: 'https://52.12.176.90:1443/',
                timeout: 30000
            },
            proofs: true,
            debug: import.meta.env.DEV,
            ...options
        };

        // Create global SDK instance
        const sdk = new WasmSDK(config);
        
        // Make available globally
        app.config.globalProperties.$wasmSDK = sdk;
        
        // Provide for composition API
        app.provide('wasmSDK', sdk);
        
        // Initialize when app is mounted
        app.mixin({
            async mounted() {
                if (this.$root === this && !sdk.isInitialized()) {
                    try {
                        await sdk.initialize();
                        console.log('Global WASM SDK initialized');
                    } catch (error) {
                        console.error('Global WASM SDK initialization failed:', error);
                    }
                }
            },
            
            async unmounted() {
                if (this.$root === this) {
                    await sdk.destroy();
                }
            }
        });
    }
};
```

### Using the Plugin

```javascript
// main.js
import { createApp } from 'vue';
import App from './App.vue';
import WasmSDKPlugin from './plugins/wasmSDK';

const app = createApp(App);

// Install the plugin
app.use(WasmSDKPlugin, {
    network: 'testnet',
    debug: true
});

app.mount('#app');
```

## Component Examples

### Identity Display Component

```vue
<!-- components/IdentityDisplay.vue -->
<template>
  <div class="identity-display">
    <h3>Identity Display</h3>
    
    <div v-if="loading" class="loading">
      <span>Loading identity...</span>
    </div>
    
    <div v-else-if="error" class="error">
      <h4>Error loading identity</h4>
      <p>{{ error.message }}</p>
      <button @click="refresh">Retry</button>
    </div>
    
    <div v-else-if="identity" class="identity-details">
      <div class="identity-field">
        <label>ID:</label>
        <span class="monospace">{{ identityId }}</span>
      </div>
      
      <div class="identity-field">
        <label>Public Keys:</label>
        <span>{{ identity.publicKeys?.length || 0 }}</span>
      </div>
      
      <div class="identity-field">
        <label>Balance:</label>
        <span>{{ formatBalance(identity.balance) }} DASH</span>
      </div>
      
      <div class="identity-field">
        <label>Revision:</label>
        <span>{{ identity.revision }}</span>
      </div>
    </div>
    
    <div v-else class="not-found">
      Identity not found
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed } from 'vue';
import { useWasmSDKAdvanced } from '../composables/useWasmSDKAdvanced';

const props = defineProps({
  identityId: {
    type: String,
    required: true
  },
  config: {
    type: Object,
    default: () => ({
      network: 'testnet',
      transport: { url: 'https://52.12.176.90:1443/' }
    })
  }
});

const emit = defineEmits(['error', 'loaded']);

// Use the advanced SDK composable
const { sdk, isReady, error: sdkError } = useWasmSDKAdvanced(props.config);

// Component state
const identity = ref(null);
const loading = ref(false);
const error = ref(null);

// Computed
const formatBalance = computed(() => (balance) => {
  return ((balance || 0) / 100000000).toFixed(8);
});

// Methods
const fetchIdentity = async () => {
  if (!isReady.value || !props.identityId) return;
  
  try {
    loading.value = true;
    error.value = null;
    
    const result = await sdk.value.getIdentity(props.identityId);
    identity.value = result;
    
    emit('loaded', result);
  } catch (err) {
    error.value = err;
    emit('error', err);
  } finally {
    loading.value = false;
  }
};

const refresh = () => {
  identity.value = null;
  fetchIdentity();
};

// Watchers
watch(isReady, (ready) => {
  if (ready) fetchIdentity();
});

watch(() => props.identityId, () => {
  if (isReady.value) fetchIdentity();
});

watch(sdkError, (err) => {
  if (err) {
    error.value = err;
    emit('error', err);
  }
});
</script>

<style scoped>
.identity-display {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 1rem;
  margin: 1rem 0;
}

.loading {
  text-align: center;
  padding: 2rem;
  color: #666;
}

.error {
  color: #dc3545;
  padding: 1rem;
  background: #f8d7da;
  border-radius: 4px;
}

.identity-details {
  display: grid;
  gap: 0.5rem;
}

.identity-field {
  display: flex;
  justify-content: space-between;
  padding: 0.25rem 0;
  border-bottom: 1px solid #eee;
}

.identity-field label {
  font-weight: bold;
  color: #333;
}

.monospace {
  font-family: 'Courier New', monospace;
  font-size: 0.9em;
  background: #f5f5f5;
  padding: 0.2em 0.4em;
  border-radius: 3px;
}

.not-found {
  text-align: center;
  color: #666;
  padding: 2rem;
}
</style>
```

### Document List Component

```vue
<!-- components/DocumentList.vue -->
<template>
  <div class="document-list">
    <div class="header">
      <h3>Documents ({{ documentType }})</h3>
      <div class="controls">
        <button @click="refresh" :disabled="loading">
          Refresh
        </button>
        <select v-model="pageSize" @change="refresh">
          <option :value="10">10 per page</option>
          <option :value="25">25 per page</option>
          <option :value="50">50 per page</option>
        </select>
      </div>
    </div>

    <div v-if="loading" class="loading">
      Loading documents...
    </div>

    <div v-else-if="error" class="error">
      <h4>Error loading documents</h4>
      <p>{{ error.message }}</p>
      <button @click="refresh">Retry</button>
    </div>

    <div v-else-if="documents.length === 0" class="empty">
      No documents found
    </div>

    <div v-else class="documents">
      <div 
        v-for="(document, index) in documents" 
        :key="document.id || index"
        class="document-item"
      >
        <div class="document-header">
          <h4>Document #{{ (currentPage - 1) * pageSize + index + 1 }}</h4>
          <span class="document-id">{{ document.id }}</span>
        </div>
        
        <div class="document-meta">
          <span><strong>Owner:</strong> {{ document.ownerId }}</span>
          <span><strong>Created:</strong> {{ formatDate(document.createdAt) }}</span>
        </div>
        
        <div class="document-data">
          <pre>{{ JSON.stringify(document.data, null, 2) }}</pre>
        </div>
      </div>
    </div>

    <div v-if="totalPages > 1" class="pagination">
      <button 
        @click="prevPage" 
        :disabled="currentPage === 1 || loading"
      >
        Previous
      </button>
      
      <span class="page-info">
        Page {{ currentPage }} of {{ totalPages }}
      </span>
      
      <button 
        @click="nextPage" 
        :disabled="currentPage === totalPages || loading"
      >
        Next
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { useWasmSDKAdvanced } from '../composables/useWasmSDKAdvanced';

const props = defineProps({
  contractId: {
    type: String,
    required: true
  },
  documentType: {
    type: String,
    required: true
  },
  ownerId: String,
  config: {
    type: Object,
    default: () => ({
      network: 'testnet',
      transport: { url: 'https://52.12.176.90:1443/' }
    })
  }
});

const emit = defineEmits(['error', 'loaded']);

// Use SDK
const { sdk, isReady } = useWasmSDKAdvanced(props.config);

// Component state
const documents = ref([]);
const loading = ref(false);
const error = ref(null);
const currentPage = ref(1);
const pageSize = ref(10);
const totalDocuments = ref(0);

// Computed
const totalPages = computed(() => Math.ceil(totalDocuments.value / pageSize.value));

// Methods
const fetchDocuments = async () => {
  if (!isReady.value || !props.contractId) return;

  try {
    loading.value = true;
    error.value = null;

    const query = {
      limit: pageSize.value,
      offset: (currentPage.value - 1) * pageSize.value
    };

    if (props.ownerId) {
      query.where = [['ownerId', '=', props.ownerId]];
    }

    const results = await sdk.value.getDocuments(
      props.contractId,
      props.documentType,
      query
    );

    documents.value = results;
    totalDocuments.value = results.length; // This would ideally come from a count query
    
    emit('loaded', results);
  } catch (err) {
    error.value = err;
    emit('error', err);
  } finally {
    loading.value = false;
  }
};

const refresh = () => {
  currentPage.value = 1;
  fetchDocuments();
};

const nextPage = () => {
  if (currentPage.value < totalPages.value) {
    currentPage.value++;
    fetchDocuments();
  }
};

const prevPage = () => {
  if (currentPage.value > 1) {
    currentPage.value--;
    fetchDocuments();
  }
};

const formatDate = (timestamp) => {
  if (!timestamp) return 'N/A';
  return new Date(timestamp).toLocaleString();
};

// Watchers
watch(isReady, (ready) => {
  if (ready) fetchDocuments();
});

watch([() => props.contractId, () => props.documentType, () => props.ownerId], () => {
  if (isReady.value) refresh();
});
</script>

<style scoped>
.document-list {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 1rem;
  margin: 1rem 0;
}

.header {
  display: flex;
  justify-content: between;
  align-items: center;
  margin-bottom: 1rem;
  border-bottom: 1px solid #eee;
  padding-bottom: 1rem;
}

.controls {
  display: flex;
  gap: 1rem;
  align-items: center;
}

.documents {
  display: grid;
  gap: 1rem;
}

.document-item {
  border: 1px solid #eee;
  border-radius: 4px;
  padding: 1rem;
  background: #f9f9f9;
}

.document-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.document-id {
  font-family: monospace;
  font-size: 0.8em;
  background: #e9ecef;
  padding: 0.2em 0.4em;
  border-radius: 3px;
}

.document-meta {
  display: flex;
  gap: 1rem;
  margin-bottom: 0.5rem;
  font-size: 0.9em;
  color: #666;
}

.document-data pre {
  background: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 4px;
  padding: 0.5rem;
  font-size: 0.8em;
  overflow-x: auto;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid #eee;
}

.page-info {
  font-weight: bold;
}

.loading, .error, .empty {
  text-align: center;
  padding: 2rem;
}

.error {
  color: #dc3545;
  background: #f8d7da;
  border-radius: 4px;
}
</style>
```

## State Management with Pinia

### SDK Store

```javascript
// stores/wasmSDK.js
import { defineStore } from 'pinia';
import { WasmSDK, WasmTransportError } from '@dashevo/dash-wasm-sdk';

export const useWasmSDKStore = defineStore('wasmSDK', {
  state: () => ({
    sdk: null,
    loading: false,
    error: null,
    initialized: false,
    connected: false,
    config: {
      network: 'testnet',
      transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
      },
      proofs: true
    },
    retryCount: 0,
    maxRetries: 3
  }),

  getters: {
    isReady: (state) => state.initialized && !state.loading && !state.error,
    canRetry: (state) => state.error && state.retryCount < state.maxRetries,
    connectionStatus: (state) => {
      if (state.loading) return 'connecting';
      if (state.error) return 'error';
      if (state.connected) return 'connected';
      return 'disconnected';
    }
  },

  actions: {
    async initialize(customConfig = null) {
      if (this.sdk && this.initialized) return;

      try {
        this.loading = true;
        this.error = null;
        
        const config = customConfig || this.config;
        this.sdk = new WasmSDK(config);
        await this.sdk.initialize();
        
        this.initialized = true;
        this.connected = true;
        this.retryCount = 0;
      } catch (error) {
        this.error = error;
        this.connected = false;
        
        // Auto-retry for transport errors
        if (error instanceof WasmTransportError && this.retryCount < this.maxRetries) {
          this.retryCount++;
          setTimeout(() => {
            this.initialize();
          }, 1000 * this.retryCount);
        }
      } finally {
        this.loading = false;
      }
    },

    async destroy() {
      if (this.sdk) {
        await this.sdk.destroy();
        this.sdk = null;
      }
      
      this.initialized = false;
      this.connected = false;
      this.error = null;
      this.retryCount = 0;
    },

    async retry() {
      if (this.canRetry) {
        await this.destroy();
        await this.initialize();
      }
    },

    updateConfig(newConfig) {
      this.config = { ...this.config, ...newConfig };
    }
  }
});
```

### Using the Store in Components

```vue
<!-- components/SDKStatus.vue -->
<template>
  <div class="sdk-status" :class="statusClass">
    <div class="status-indicator">
      <span class="status-dot"></span>
      {{ connectionStatus }}
    </div>
    
    <div v-if="loading" class="status-message">
      Initializing SDK...
    </div>
    
    <div v-else-if="error" class="status-message error">
      {{ error.message }}
      <button v-if="canRetry" @click="retry" class="retry-btn">
        Retry ({{ retryCount }}/{{ maxRetries }})
      </button>
    </div>
    
    <div v-else-if="isReady" class="status-message">
      Network: {{ config.network }}
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { useWasmSDKStore } from '../stores/wasmSDK';

const store = useWasmSDKStore();
const { 
  loading, 
  error, 
  config, 
  retryCount, 
  maxRetries 
} = storeToRefs(store);

const { 
  isReady, 
  canRetry, 
  connectionStatus 
} = storeToRefs(store);

const statusClass = computed(() => ({
  'status-connecting': loading.value,
  'status-connected': isReady.value,
  'status-error': error.value
}));

const retry = () => {
  store.retry();
};

onMounted(() => {
  if (!store.initialized) {
    store.initialize();
  }
});
</script>

<style scoped>
.sdk-status {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  border: 1px solid #ddd;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: bold;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6c757d;
}

.status-connecting .status-dot {
  background: #ffc107;
  animation: pulse 1.5s ease-in-out infinite;
}

.status-connected .status-dot {
  background: #28a745;
}

.status-error .status-dot {
  background: #dc3545;
}

.status-message.error {
  color: #dc3545;
}

.retry-btn {
  margin-left: 0.5rem;
  padding: 0.2rem 0.5rem;
  font-size: 0.8rem;
  border: 1px solid #dc3545;
  background: transparent;
  color: #dc3545;
  border-radius: 3px;
  cursor: pointer;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
```

## Provide/Inject Pattern

### App-level Provider

```vue
<!-- App.vue -->
<template>
  <div id="app">
    <Header />
    <main>
      <router-view />
    </main>
    <Footer />
  </div>
</template>

<script setup>
import { provide, onMounted, onUnmounted } from 'vue';
import { useWasmSDKAdvanced } from './composables/useWasmSDKAdvanced';
import Header from './components/Header.vue';
import Footer from './components/Footer.vue';

const config = {
  network: import.meta.env.VITE_NETWORK || 'testnet',
  transport: {
    url: import.meta.env.VITE_SDK_URL || 'https://52.12.176.90:1443/',
    timeout: 30000
  },
  proofs: true,
  debug: import.meta.env.DEV
};

const sdkComposable = useWasmSDKAdvanced(config);

// Provide SDK to all child components
provide('wasmSDK', sdkComposable);
provide('wasmSDKConfig', config);
</script>
```

### Child Component Using Inject

```vue
<!-- components/IdentitySearch.vue -->
<template>
  <div class="identity-search">
    <h2>Identity Search</h2>
    
    <form @submit.prevent="searchIdentity">
      <div class="input-group">
        <input
          v-model="identityId"
          type="text"
          placeholder="Enter identity ID..."
          :disabled="!isReady"
        />
        <button type="submit" :disabled="!isReady || loading">
          {{ loading ? 'Searching...' : 'Search' }}
        </button>
      </div>
    </form>

    <div v-if="searchError" class="error">
      {{ searchError.message }}
    </div>

    <div v-if="identity" class="result">
      <h3>Identity Found</h3>
      <pre>{{ JSON.stringify(identity, null, 2) }}</pre>
    </div>
  </div>
</template>

<script setup>
import { ref, inject } from 'vue';

// Inject SDK from parent
const { sdk, isReady } = inject('wasmSDK');

// Component state
const identityId = ref('');
const identity = ref(null);
const loading = ref(false);
const searchError = ref(null);

const searchIdentity = async () => {
  if (!identityId.value.trim() || !isReady.value) return;

  try {
    loading.value = true;
    searchError.value = null;
    identity.value = null;

    const result = await sdk.value.getIdentity(identityId.value.trim());
    identity.value = result;
  } catch (error) {
    searchError.value = error;
  } finally {
    loading.value = false;
  }
};
</script>

<style scoped>
.identity-search {
  max-width: 600px;
  margin: 0 auto;
  padding: 1rem;
}

.input-group {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}

.input-group input {
  flex: 1;
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.input-group button {
  padding: 0.5rem 1rem;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.input-group button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.result {
  margin-top: 1rem;
  padding: 1rem;
  border: 1px solid #ddd;
  border-radius: 4px;
  background: #f8f9fa;
}

.result pre {
  font-size: 0.8rem;
  overflow-x: auto;
}
</style>
```

## Complete Application Example

```vue
<!-- components/Dashboard.vue -->
<template>
  <div class="dashboard">
    <header class="dashboard-header">
      <h1>Dash Platform Dashboard</h1>
      <SDKStatus />
    </header>

    <div class="dashboard-content">
      <div class="sidebar">
        <nav class="nav-menu">
          <button 
            @click="activeTab = 'identities'"
            :class="{ active: activeTab === 'identities' }"
          >
            Identities
          </button>
          <button 
            @click="activeTab = 'contracts'"
            :class="{ active: activeTab === 'contracts' }"
          >
            Contracts
          </button>
          <button 
            @click="activeTab = 'documents'"
            :class="{ active: activeTab === 'documents' }"
          >
            Documents
          </button>
        </nav>

        <div class="settings">
          <h3>Settings</h3>
          <div class="setting-group">
            <label>
              Network:
              <select v-model="selectedNetwork" @change="updateNetwork">
                <option value="testnet">Testnet</option>
                <option value="mainnet">Mainnet</option>
              </select>
            </label>
          </div>
        </div>
      </div>

      <main class="main-content">
        <component 
          :is="activeComponent"
          :config="currentConfig"
          @error="handleError"
        />
      </main>
    </div>

    <!-- Global error notification -->
    <div v-if="globalError" class="error-notification">
      <p>{{ globalError.message }}</p>
      <button @click="clearGlobalError">Dismiss</button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useWasmSDKStore } from '../stores/wasmSDK';
import SDKStatus from './SDKStatus.vue';
import IdentityTab from './tabs/IdentityTab.vue';
import ContractTab from './tabs/ContractTab.vue';
import DocumentTab from './tabs/DocumentTab.vue';

// Store
const store = useWasmSDKStore();
const { config } = storeToRefs(store);

// Component state
const activeTab = ref('identities');
const selectedNetwork = ref(config.value.network);
const globalError = ref(null);

// Computed
const currentConfig = computed(() => ({
  ...config.value,
  network: selectedNetwork.value
}));

const activeComponent = computed(() => {
  switch (activeTab.value) {
    case 'identities': return IdentityTab;
    case 'contracts': return ContractTab;
    case 'documents': return DocumentTab;
    default: return IdentityTab;
  }
});

// Methods
const updateNetwork = async () => {
  try {
    store.updateConfig({ network: selectedNetwork.value });
    await store.destroy();
    await store.initialize();
  } catch (error) {
    handleError(error);
  }
};

const handleError = (error) => {
  globalError.value = error;
  setTimeout(() => {
    if (globalError.value === error) {
      globalError.value = null;
    }
  }, 5000);
};

const clearGlobalError = () => {
  globalError.value = null;
};

// Initialize store
store.initialize();
</script>

<style scoped>
.dashboard {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 2rem;
  border-bottom: 1px solid #ddd;
  background: #f8f9fa;
}

.dashboard-content {
  flex: 1;
  display: flex;
}

.sidebar {
  width: 250px;
  padding: 1rem;
  border-right: 1px solid #ddd;
  background: #f8f9fa;
}

.nav-menu {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-bottom: 2rem;
}

.nav-menu button {
  padding: 0.75rem;
  text-align: left;
  border: none;
  background: transparent;
  cursor: pointer;
  border-radius: 4px;
}

.nav-menu button:hover {
  background: #e9ecef;
}

.nav-menu button.active {
  background: #007bff;
  color: white;
}

.settings {
  border-top: 1px solid #ddd;
  padding-top: 1rem;
}

.setting-group {
  margin: 0.5rem 0;
}

.setting-group label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.9rem;
}

.setting-group select {
  padding: 0.25rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.main-content {
  flex: 1;
  padding: 1rem;
  overflow-y: auto;
}

.error-notification {
  position: fixed;
  bottom: 1rem;
  right: 1rem;
  background: #dc3545;
  color: white;
  padding: 1rem;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 1rem;
  max-width: 400px;
  box-shadow: 0 2px 10px rgba(0,0,0,0.1);
}

.error-notification button {
  background: transparent;
  border: 1px solid white;
  color: white;
  padding: 0.25rem 0.5rem;
  border-radius: 3px;
  cursor: pointer;
}
</style>
```

## TypeScript Support

### Type Definitions

```typescript
// types/wasm-sdk.ts
import { WasmSDK, WasmSDKConfig } from '@dashevo/dash-wasm-sdk';
import { Ref } from 'vue';

export interface UseWasmSDKResult {
  sdk: Ref<WasmSDK | null>;
  loading: Ref<boolean>;
  error: Ref<Error | null>;
  initialized: Ref<boolean>;
  connected?: Ref<boolean>;
  retryCount?: Ref<number>;
  isReady?: ComputedRef<boolean>;
  canRetry?: ComputedRef<boolean>;
  connectionStatus?: ComputedRef<string>;
  initialize: () => Promise<void>;
  destroy: () => Promise<void>;
  retry?: () => void;
}

export interface WasmSDKProviderOptions {
  maxRetries?: number;
  retryDelay?: number;
  autoReconnect?: boolean;
  enableResourceCleanup?: boolean;
}
```

### TypeScript Composable

```typescript
// composables/useWasmSDK.ts
import { ref, computed, onMounted, onUnmounted, watch, readonly, Ref } from 'vue';
import { WasmSDK, WasmSDKConfig, WasmTransportError } from '@dashevo/dash-wasm-sdk';
import type { UseWasmSDKResult, WasmSDKProviderOptions } from '../types/wasm-sdk';

export function useWasmSDKAdvanced(
  config: WasmSDKConfig,
  options: WasmSDKProviderOptions = {}
): UseWasmSDKResult {
  const {
    maxRetries = 3,
    retryDelay = 1000,
    autoReconnect = true,
    enableResourceCleanup = true
  } = options;

  // Reactive state
  const sdk: Ref<WasmSDK | null> = ref(null);
  const loading = ref(false);
  const error: Ref<Error | null> = ref(null);
  const initialized = ref(false);
  const connected = ref(false);
  const retryCount = ref(0);

  let sdkInstance: WasmSDK | null = null;
  let retryTimeout: NodeJS.Timeout | null = null;
  let cleanupInterval: NodeJS.Timeout | null = null;

  // Computed properties
  const isReady = computed(() => initialized.value && !loading.value && !error.value);
  const canRetry = computed(() => error.value !== null && retryCount.value < maxRetries);
  const connectionStatus = computed(() => {
    if (loading.value) return 'connecting';
    if (error.value) return 'error';
    if (connected.value) return 'connected';
    return 'disconnected';
  });

  const initialize = async (attempt = 1): Promise<void> => {
    if (sdkInstance && initialized.value) return;

    try {
      loading.value = true;
      error.value = null;
      
      sdkInstance = new WasmSDK(config);
      await sdkInstance.initialize();
      
      sdk.value = sdkInstance;
      initialized.value = true;
      connected.value = true;
      retryCount.value = 0;

      if (enableResourceCleanup) {
        cleanupInterval = setInterval(() => {
          if (sdkInstance) {
            sdkInstance.cleanupResources();
          }
        }, 300000);
      }
    } catch (err) {
      const error = err as Error;
      error.value = err;
      connected.value = false;
      
      if (err instanceof WasmTransportError && attempt < maxRetries && autoReconnect) {
        retryCount.value = attempt;
        const delay = retryDelay * attempt;
        
        retryTimeout = setTimeout(() => {
          initialize(attempt + 1);
        }, delay);
      }
    } finally {
      loading.value = false;
    }
  };

  const destroy = async (): Promise<void> => {
    if (retryTimeout) {
      clearTimeout(retryTimeout);
      retryTimeout = null;
    }
    if (cleanupInterval) {
      clearInterval(cleanupInterval);
      cleanupInterval = null;
    }

    if (sdkInstance) {
      await sdkInstance.destroy();
      sdkInstance = null;
    }

    sdk.value = null;
    initialized.value = false;
    connected.value = false;
    error.value = null;
    retryCount.value = 0;
  };

  const retry = (): void => {
    if (canRetry.value) {
      initialize(1);
    }
  };

  onMounted(initialize);
  onUnmounted(destroy);

  watch(() => JSON.stringify(config), async () => {
    await destroy();
    await initialize();
  });

  return {
    sdk: readonly(sdk),
    loading: readonly(loading),
    error: readonly(error),
    initialized: readonly(initialized),
    connected: readonly(connected),
    retryCount: readonly(retryCount),
    isReady,
    canRetry,
    connectionStatus,
    initialize,
    destroy,
    retry
  };
}
```

## SSR/Nuxt.js Integration

### Nuxt Plugin

```javascript
// plugins/wasmSDK.client.js
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

export default defineNuxtPlugin(async () => {
  // Only run on client side
  if (process.server) return;

  const config = useRuntimeConfig();
  
  const sdk = new WasmSDK({
    network: config.public.wasmSdkNetwork || 'testnet',
    transport: {
      url: config.public.wasmSdkUrl || 'https://52.12.176.90:1443/',
      timeout: 30000
    },
    proofs: true,
    debug: config.public.wasmSdkDebug || false
  });

  return {
    provide: {
      wasmSDK: sdk
    }
  };
});
```

### Nuxt Configuration

```javascript
// nuxt.config.ts
export default defineNuxtConfig({
  plugins: [
    '~/plugins/wasmSDK.client.js'
  ],
  
  runtimeConfig: {
    public: {
      wasmSdkNetwork: process.env.WASM_SDK_NETWORK || 'testnet',
      wasmSdkUrl: process.env.WASM_SDK_URL || 'https://52.12.176.90:1443/',
      wasmSdkDebug: process.env.NODE_ENV === 'development'
    }
  },

  ssr: true,

  // Disable SSR for WASM-heavy components
  components: [
    {
      path: '~/components',
      global: true
    }
  ]
});
```

### SSR-Safe Component

```vue
<!-- components/ClientOnlySDK.vue -->
<template>
  <ClientOnly>
    <div class="sdk-component">
      <IdentityDisplay :identity-id="identityId" />
    </div>
    <template #fallback>
      <div class="ssr-fallback">
        Loading SDK components...
      </div>
    </template>
  </ClientOnly>
</template>

<script setup>
import IdentityDisplay from './IdentityDisplay.vue';

const props = defineProps({
  identityId: {
    type: String,
    required: true
  }
});
</script>
```

## Troubleshooting

### Common Vue-Specific Issues

#### 1. Reactivity Issues with WASM Objects

```javascript
// Problem: WASM objects are not reactive
const identity = ref(wasmIdentityObject); // Not reactive

// Solution: Extract data or use reactive wrappers
const identity = ref({
  id: wasmIdentityObject.getId(),
  balance: wasmIdentityObject.getBalance(),
  // ... other properties
});
```

#### 2. Memory Leaks in Long-Running Apps

```javascript
// Solution: Implement proper cleanup
onUnmounted(() => {
  if (sdkInstance) {
    sdkInstance.destroy();
  }
});

// Use watchers for cleanup
watchEffect((onInvalidate) => {
  const sdk = new WasmSDK(config);
  
  onInvalidate(() => {
    sdk.destroy();
  });
});
```

#### 3. SSR Hydration Mismatches

```vue
<!-- Problem: Different content on server vs client -->
<div>{{ sdk ? 'SDK Ready' : 'Loading...' }}</div>

<!-- Solution: Use ClientOnly or SSR-safe conditions -->
<ClientOnly>
  <div>{{ sdk ? 'SDK Ready' : 'Loading...' }}</div>
  <template #fallback>
    <div>Loading...</div>
  </template>
</ClientOnly>
```

This Vue.js integration provides a comprehensive foundation for building reactive Dash Platform applications with proper state management, error handling, and SSR support.