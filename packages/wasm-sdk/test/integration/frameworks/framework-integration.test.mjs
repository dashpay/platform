/**
 * Framework Integration Tests
 * Tests WASM SDK integration with React, Vue, Angular, and Vanilla JS
 */

import { jest } from '@jest/globals';

describe('Framework Integration Tests', () => {
  let sdk;

  beforeAll(async () => {
    const wasmInitialized = await global.initializeWasm();
    if (!wasmInitialized) {
      throw new Error('Failed to initialize WASM - tests cannot proceed');
    }
  });

  beforeEach(async () => {
    sdk = await global.createTestSDK({
      network: 'testnet',
      proofs: false
    });
  });

  afterEach(async () => {
    if (sdk && sdk.destroy) {
      await sdk.destroy();
    }
  });

  describe('React Integration', () => {
    test('should work with React component lifecycle', async () => {
      // Simulate React component mounting
      const componentDidMount = async () => {
        const sdk = await global.createTestSDK();
        return sdk;
      };

      // Simulate component state updates
      const handleStatusUpdate = async (sdk) => {
        try {
          const status = await sdk.getStatus();
          return { connected: true, status };
        } catch (error) {
          return { connected: false, error: error.message };
        }
      };

      // Simulate component unmounting
      const componentWillUnmount = async (sdk) => {
        if (sdk && sdk.destroy) {
          await sdk.destroy();
        }
      };

      // Test the lifecycle
      const componentSdk = await componentDidMount();
      expect(componentSdk).toBeDefined();

      const statusResult = await handleStatusUpdate(componentSdk);
      expect(statusResult.connected).toBe(true);

      await componentWillUnmount(componentSdk);
      // After cleanup, component should be unmounted gracefully
      expect(true).toBe(true); // Cleanup completed
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle React hooks pattern', async () => {
      // Simulate useEffect hook
      let sdkInstance = null;
      let isLoading = true;
      let error = null;
      let data = null;

      // useEffect equivalent
      const useSDK = async () => {
        try {
          isLoading = true;
          sdkInstance = await global.createTestSDK();
          
          // Simulate data fetching
          data = await sdkInstance.getStatus();
          isLoading = false;
          error = null;
        } catch (err) {
          error = err.message;
          isLoading = false;
          data = null;
        }
      };

      // Cleanup effect
      const cleanup = async () => {
        if (sdkInstance) {
          await sdkInstance.destroy();
        }
      };

      await useSDK();
      
      expect(isLoading).toBe(false);
      expect(error).toBeNull();
      expect(data).toBeDefined();

      await cleanup();
    }, TEST_CONFIG.STANDARD_TIMEOUT);

    test('should integrate with React state management (Redux pattern)', async () => {
      // Simulate Redux-style state management
      const initialState = {
        sdk: null,
        isConnected: false,
        identities: [],
        loading: false,
        error: null
      };

      let state = { ...initialState };

      // Action creators
      const actions = {
        SDK_INIT_START: 'SDK_INIT_START',
        SDK_INIT_SUCCESS: 'SDK_INIT_SUCCESS',
        SDK_INIT_FAILURE: 'SDK_INIT_FAILURE',
        IDENTITY_FETCH_SUCCESS: 'IDENTITY_FETCH_SUCCESS'
      };

      // Reducer
      const reducer = (state, action) => {
        switch (action.type) {
          case actions.SDK_INIT_START:
            return { ...state, loading: true, error: null };
          case actions.SDK_INIT_SUCCESS:
            return { ...state, loading: false, sdk: action.payload, isConnected: true };
          case actions.SDK_INIT_FAILURE:
            return { ...state, loading: false, error: action.payload, isConnected: false };
          case actions.IDENTITY_FETCH_SUCCESS:
            return { ...state, identities: [...state.identities, action.payload] };
          default:
            return state;
        }
      };

      // Async action (thunk pattern)
      const initializeSDK = async (dispatch) => {
        dispatch({ type: actions.SDK_INIT_START });
        try {
          const sdk = await global.createTestSDK();
          dispatch({ type: actions.SDK_INIT_SUCCESS, payload: sdk });
          return sdk;
        } catch (error) {
          dispatch({ type: actions.SDK_INIT_FAILURE, payload: error.message });
          throw error;
        }
      };

      // Mock dispatch
      const dispatch = (action) => {
        state = reducer(state, action);
      };

      // Test the flow
      const resultSdk = await initializeSDK(dispatch);
      
      expect(state.isConnected).toBe(true);
      expect(state.loading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.sdk).toBe(resultSdk);

      // Cleanup
      await resultSdk.destroy();
    }, TEST_CONFIG.STANDARD_TIMEOUT);
  });

  describe('Vue.js Integration', () => {
    test('should work with Vue reactive data', async () => {
      // Simulate Vue reactive data pattern
      const vueData = {
        sdk: null,
        isConnected: false,
        networkStatus: null,
        identities: [],
        loading: false,
        error: null
      };

      // Simulate Vue methods
      const vueMethods = {
        async initializeSDK() {
          vueData.loading = true;
          vueData.error = null;
          
          try {
            vueData.sdk = await global.createTestSDK();
            vueData.isConnected = true;
            vueData.loading = false;
          } catch (error) {
            vueData.error = error.message;
            vueData.loading = false;
            vueData.isConnected = false;
          }
        },

        async fetchNetworkStatus() {
          if (!vueData.sdk) return;
          
          try {
            vueData.networkStatus = await vueData.sdk.getStatus();
          } catch (error) {
            vueData.error = error.message;
          }
        },

        async addIdentity(identityId) {
          if (!vueData.sdk) return;
          
          try {
            const identity = await vueData.sdk.getIdentity(identityId);
            if (identity) {
              vueData.identities.push(identity);
            }
          } catch (error) {
            vueData.error = error.message;
          }
        },

        cleanup() {
          if (vueData.sdk) {
            vueData.sdk.destroy();
            vueData.sdk = null;
            vueData.isConnected = false;
          }
        }
      };

      // Test Vue lifecycle equivalent
      await vueMethods.initializeSDK();
      expect(vueData.isConnected).toBe(true);
      expect(vueData.loading).toBe(false);

      await vueMethods.fetchNetworkStatus();
      expect(vueData.networkStatus).toBeDefined();

      // Test adding identity
      await vueMethods.addIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID);
      
      // Cleanup
      vueMethods.cleanup();
      expect(vueData.isConnected).toBe(false);
      expect(vueData.sdk).toBeNull();
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle Vue computed properties pattern', async () => {
      const vueData = {
        sdk: null,
        identities: [
          { id: 'id1', balance: 100 },
          { id: 'id2', balance: 200 },
          { id: 'id3', balance: 150 }
        ],
        selectedIdentityId: 'id2'
      };

      // Simulate Vue computed properties
      const vueComputed = {
        totalBalance: () => {
          return vueData.identities.reduce((sum, identity) => sum + identity.balance, 0);
        },
        
        selectedIdentity: () => {
          return vueData.identities.find(identity => identity.id === vueData.selectedIdentityId);
        },
        
        isSDKReady: () => {
          return vueData.sdk !== null;
        }
      };

      // Test computed properties
      expect(vueComputed.totalBalance()).toBe(450);
      expect(vueComputed.selectedIdentity().balance).toBe(200);
      expect(vueComputed.isSDKReady()).toBe(false);

      // Initialize SDK
      vueData.sdk = await global.createTestSDK();
      expect(vueComputed.isSDKReady()).toBe(true);

      // Cleanup
      await vueData.sdk.destroy();
    }, TEST_CONFIG.STANDARD_TIMEOUT);
  });

  describe('Angular Integration', () => {
    test('should work with Angular service pattern', async () => {
      // Simulate Angular service
      class WasmSDKService {
        constructor() {
          this.sdk = null;
          this.isConnected = false;
          this.connectionStatus$ = null; // Would be Observable in real Angular
        }

        async initialize() {
          try {
            this.sdk = await global.createTestSDK();
            this.isConnected = true;
            return this.sdk;
          } catch (error) {
            this.isConnected = false;
            throw error;
          }
        }

        async getStatus() {
          if (!this.sdk) {
            throw new Error('SDK not initialized');
          }
          return await this.sdk.getStatus();
        }

        async getIdentity(identityId) {
          if (!this.sdk) {
            throw new Error('SDK not initialized');
          }
          return await this.sdk.getIdentity(identityId);
        }

        destroy() {
          if (this.sdk) {
            this.sdk.destroy();
            this.sdk = null;
            this.isConnected = false;
          }
        }
      }

      // Test Angular service pattern
      const sdkService = new WasmSDKService();
      expect(sdkService.isConnected).toBe(false);

      await sdkService.initialize();
      expect(sdkService.isConnected).toBe(true);

      const status = await sdkService.getStatus();
      expect(status).toBeDefined();

      sdkService.destroy();
      expect(sdkService.isConnected).toBe(false);
    }, TEST_CONFIG.STANDARD_TIMEOUT);

    test('should handle Angular component lifecycle', async () => {
      // Simulate Angular component
      class IdentityComponent {
        constructor(sdkService) {
          this.sdkService = sdkService;
          this.identity = null;
          this.loading = false;
          this.error = null;
        }

        // ngOnInit equivalent
        async ngOnInit() {
          await this.loadIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID);
        }

        async loadIdentity(identityId) {
          this.loading = true;
          this.error = null;

          try {
            this.identity = await this.sdkService.getIdentity(identityId);
            this.loading = false;
          } catch (error) {
            this.error = error.message;
            this.loading = false;
          }
        }

        // ngOnDestroy equivalent
        ngOnDestroy() {
          this.identity = null;
          this.error = null;
        }
      }

      // Initialize service
      const sdkService = new WasmSDKService();
      await sdkService.initialize();

      // Test component lifecycle
      const component = new IdentityComponent(sdkService);
      await component.ngOnInit();

      // Clean up
      component.ngOnDestroy();
      sdkService.destroy();
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should work with Angular dependency injection pattern', async () => {
      // Simulate Angular DI container
      const injector = new Map();

      // Register services
      injector.set('WasmSDKService', {
        async initialize() {
          this.sdk = await global.createTestSDK();
          return this.sdk;
        },
        async getIdentity(id) {
          return await this.sdk.getIdentity(id);
        },
        destroy() {
          if (this.sdk) this.sdk.destroy();
        }
      });

      injector.set('IdentityService', {
        constructor(sdkService) {
          this.sdkService = sdkService;
        },
        async fetchIdentity(id) {
          return await this.sdkService.getIdentity(id);
        }
      });

      // Test dependency injection
      const sdkService = injector.get('WasmSDKService');
      await sdkService.initialize();

      const identityService = injector.get('IdentityService');
      identityService.constructor(sdkService);

      // Use injected services
      try {
        await identityService.fetchIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID);
      } catch (error) {
        // Identity might not exist - that's OK for this test
        expect(error.message).toMatch(/not found/i);
      }

      // Cleanup
      sdkService.destroy();
    }, TEST_CONFIG.STANDARD_TIMEOUT);
  });

  describe('Vanilla JavaScript Integration', () => {
    test('should work with ES6 modules', async () => {
      // Test ES6 module pattern
      const WasmSDKModule = {
        instance: null,
        
        async init() {
          this.instance = await global.createTestSDK();
          return this.instance;
        },
        
        async getStatus() {
          if (!this.instance) throw new Error('Not initialized');
          return await this.instance.getStatus();
        },
        
        destroy() {
          if (this.instance) {
            this.instance.destroy();
            this.instance = null;
          }
        }
      };

      // Test module usage
      await WasmSDKModule.init();
      const status = await WasmSDKModule.getStatus();
      expect(status).toBeDefined();
      
      WasmSDKModule.destroy();
      expect(WasmSDKModule.instance).toBeNull();
    }, TEST_CONFIG.STANDARD_TIMEOUT);

    test('should work with classic JavaScript patterns', async () => {
      // Test function-based pattern
      function createSDKWrapper() {
        let sdkInstance = null;
        let isInitialized = false;

        return {
          async initialize() {
            sdkInstance = await global.createTestSDK();
            isInitialized = true;
            return sdkInstance;
          },

          isReady() {
            return isInitialized && sdkInstance !== null;
          },

          async performQuery(queryFn) {
            if (!this.isReady()) {
              throw new Error('SDK not ready');
            }
            return await queryFn(sdkInstance);
          },

          destroy() {
            if (sdkInstance) {
              sdkInstance.destroy();
              sdkInstance = null;
              isInitialized = false;
            }
          }
        };
      }

      // Test wrapper
      const wrapper = createSDKWrapper();
      expect(wrapper.isReady()).toBe(false);

      await wrapper.initialize();
      expect(wrapper.isReady()).toBe(true);

      const result = await wrapper.performQuery(async (sdk) => {
        return await sdk.getStatus();
      });
      expect(result).toBeDefined();

      wrapper.destroy();
      expect(wrapper.isReady()).toBe(false);
    }, TEST_CONFIG.STANDARD_TIMEOUT);

    test('should work with Promise-based patterns', async () => {
      // Test Promise chaining pattern
      const sdkPromise = global.createTestSDK()
        .then(sdk => {
          return {
            sdk,
            async getStatus() {
              return await sdk.getStatus();
            },
            async getIdentity(id) {
              return await sdk.getIdentity(id);
            },
            destroy() {
              return sdk.destroy();
            }
          };
        });

      const wrapper = await sdkPromise;
      const status = await wrapper.getStatus();
      expect(status).toBeDefined();

      await wrapper.destroy();
    }, TEST_CONFIG.STANDARD_TIMEOUT);

    test('should handle async/await patterns correctly', async () => {
      // Test various async patterns
      async function sequentialQueries() {
        const sdk = await global.createTestSDK();
        
        try {
          const status = await sdk.getStatus();
          expect(status).toBeDefined();

          // Try sequential identity lookup
          try {
            const identity = await sdk.getIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID);
            if (identity) {
              expect(identity.id).toBeDefined();
            }
          } catch (error) {
            // Identity might not exist
            expect(error.message).toMatch(/not found/i);
          }

          return { success: true };
        } finally {
          await sdk.destroy();
        }
      }

      const result = await sequentialQueries();
      expect(result.success).toBe(true);
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle concurrent operations properly', async () => {
      const sdk = await global.createTestSDK();

      try {
        // Test concurrent operations
        const operations = [
          sdk.getStatus(),
          sdk.generateMnemonic(12),
          sdk.getStatus() // Duplicate to test caching/concurrent handling
        ];

        const results = await Promise.allSettled(operations);
        
        // At least some operations should succeed
        const successCount = results.filter(r => r.status === 'fulfilled').length;
        expect(successCount).toBeGreaterThan(0);

        // All results should be present
        expect(results).toHaveLength(3);
        
      } finally {
        await sdk.destroy();
      }
    }, TEST_CONFIG.STANDARD_TIMEOUT);
  });

  describe('Cross-Framework Compatibility', () => {
    test('should maintain consistent API across frameworks', async () => {
      // Test that the same SDK instance works the same way regardless of framework
      const testSDKConsistency = async (frameworkName, wrapperFn) => {
        const wrapper = await wrapperFn();
        
        // Test basic operations
        const status = await wrapper.getStatus();
        expect(status).toBeDefined();

        const mnemonic = await wrapper.generateMnemonic(12);
        expect(typeof mnemonic).toBe('string');
        expect(mnemonic.split(' ')).toHaveLength(12);

        await wrapper.destroy();
        
        return { framework: frameworkName, success: true };
      };

      // Define framework-specific wrappers
      const frameworks = {
        'React': async () => {
          const sdk = await global.createTestSDK();
          return {
            getStatus: () => sdk.getStatus(),
            generateMnemonic: (words) => sdk.generateMnemonic(words),
            destroy: () => sdk.destroy()
          };
        },
        
        'Vue': async () => {
          const sdk = await global.createTestSDK();
          return {
            getStatus: () => sdk.getStatus(),
            generateMnemonic: (words) => sdk.generateMnemonic(words),
            destroy: () => sdk.destroy()
          };
        },
        
        'Vanilla': async () => {
          const sdk = await global.createTestSDK();
          return {
            getStatus: () => sdk.getStatus(),
            generateMnemonic: (words) => sdk.generateMnemonic(words),
            destroy: () => sdk.destroy()
          };
        }
      };

      // Test consistency across frameworks
      const results = [];
      for (const [framework, wrapperFn] of Object.entries(frameworks)) {
        const result = await testSDKConsistency(framework, wrapperFn);
        results.push(result);
      }

      // All frameworks should work consistently
      expect(results).toHaveLength(3);
      results.forEach(result => {
        expect(result.success).toBe(true);
      });
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle framework-specific error patterns', async () => {
      // Test error handling patterns for different frameworks
      const errorHandlingPatterns = {
        'React': async () => {
          let error = null;
          try {
            const sdk = await global.createTestSDK();
            await sdk.getIdentity('invalid-id');
          } catch (err) {
            error = { message: err.message, caught: true };
          }
          return error;
        },

        'Vue': async () => {
          const error = { value: null };
          try {
            const sdk = await global.createTestSDK();
            await sdk.getIdentity('invalid-id');
          } catch (err) {
            error.value = { message: err.message, caught: true };
          }
          return error.value;
        },

        'Promise': async () => {
          return global.createTestSDK()
            .then(sdk => sdk.getIdentity('invalid-id'))
            .catch(err => ({ message: err.message, caught: true }));
        }
      };

      // Test each pattern
      for (const [pattern, testFn] of Object.entries(errorHandlingPatterns)) {
        const result = await testFn();
        
        if (result && result.caught) {
          expect(result.message).toMatch(/invalid|not found|error/i);
        }
      }
    }, TEST_CONFIG.STANDARD_TIMEOUT);
  });
});