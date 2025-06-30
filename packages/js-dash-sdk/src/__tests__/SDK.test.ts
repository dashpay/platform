import { SDK } from '../SDK';
import * as WasmLoader from '../core/WasmLoader';
import { ContextProvider } from '../core/types';

// Mock the WASM loader
jest.mock('../core/WasmLoader');

// Mock the provider modules
jest.mock('../core/CentralizedProvider');
jest.mock('../core/EvonodesProvider');
jest.mock('../providers/CustomMasternodeProvider');

describe('SDK', () => {
  let sdk: SDK;
  const mockWasmSdk = {
    WasmSdkBuilder: {
      new_testnet: jest.fn().mockReturnValue({
        build: jest.fn().mockReturnValue({
          free: jest.fn(),
        })
      })
    },
    fetchIdentityBalance: jest.fn(),
    FetchOptions: jest.fn().mockImplementation(() => ({
      withProve: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    (WasmLoader.loadWasmSdk as jest.Mock).mockResolvedValue(mockWasmSdk);
  });

  afterEach(() => {
    if (sdk) {
      sdk.close();
    }
  });

  describe('Initialization', () => {
    it('should initialize with testnet configuration', async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();

      expect(sdk.network).toBe('testnet');
      expect(sdk.isReady()).toBe(true);
      expect(WasmLoader.loadWasmSdk).toHaveBeenCalled();
    });

    it('should initialize with mainnet configuration', async () => {
      sdk = new SDK({ network: 'mainnet' });
      await sdk.init();

      expect(sdk.network).toBe('mainnet');
      expect(sdk.isReady()).toBe(true);
    });

    it('should initialize with custom provider URL', async () => {
      const customUrl = 'https://custom-provider.example.com';
      sdk = new SDK({ 
        network: 'testnet',
        providerUrl: customUrl 
      });
      await sdk.init();

      expect(sdk.isReady()).toBe(true);
    });

    it('should throw error if already initialized', async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();

      await expect(sdk.init()).rejects.toThrow('SDK already initialized');
    });

    it('should emit ready event on successful initialization', async () => {
      sdk = new SDK({ network: 'testnet' });
      const readyHandler = jest.fn();
      sdk.on('ready', readyHandler);

      await sdk.init();

      expect(readyHandler).toHaveBeenCalled();
    });
  });

  describe('Provider Management', () => {
    beforeEach(async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();
    });

    it('should set and get current provider', () => {
      const mockProvider: ContextProvider = {
        getBlockHash: jest.fn(),
        getDataContract: jest.fn(),
        waitForStateTransitionResult: jest.fn(),
        broadcastStateTransition: jest.fn(),
        getProtocolVersion: jest.fn(),
      };

      sdk.setProvider(mockProvider);
      expect(sdk.getProvider()).toBe(mockProvider);
    });

    it('should emit provider-changed event when provider changes', () => {
      const mockProvider: ContextProvider = {
        getBlockHash: jest.fn(),
        getDataContract: jest.fn(),
        waitForStateTransitionResult: jest.fn(),
        broadcastStateTransition: jest.fn(),
        getProtocolVersion: jest.fn(),
      };

      const providerChangedHandler = jest.fn();
      sdk.on('provider-changed', providerChangedHandler);

      sdk.setProvider(mockProvider);
      expect(providerChangedHandler).toHaveBeenCalledWith(mockProvider);
    });
  });

  describe('App Registration', () => {
    beforeEach(async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();
    });

    it('should register an app', () => {
      const appName = 'TestApp';
      const contractId = 'testContractId123';

      sdk.registerApp(appName, contractId);
      
      const apps = sdk.getRegisteredApps();
      expect(apps).toHaveProperty(appName);
      expect(apps[appName]).toBe(contractId);
    });

    it('should get app contract ID', () => {
      const appName = 'TestApp';
      const contractId = 'testContractId123';

      sdk.registerApp(appName, contractId);
      
      expect(sdk.getAppContractId(appName)).toBe(contractId);
    });

    it('should return undefined for unregistered app', () => {
      expect(sdk.getAppContractId('UnknownApp')).toBeUndefined();
    });
  });

  describe('WASM SDK Access', () => {
    beforeEach(async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();
    });

    it('should provide access to WASM SDK', () => {
      const wasm = sdk.getWasmSdk();
      expect(wasm).toBe(mockWasmSdk);
    });

    it('should throw error if accessing WASM SDK before initialization', () => {
      const uninitializedSdk = new DashSDK({ network: 'testnet' });
      expect(() => uninitializedSdk.getWasmSdk()).toThrow('SDK not initialized');
    });
  });

  describe('Event Handling', () => {
    beforeEach(async () => {
      sdk = new SDK({ network: 'testnet' });
    });

    it('should handle multiple event listeners', async () => {
      const listener1 = jest.fn();
      const listener2 = jest.fn();

      sdk.on('ready', listener1);
      sdk.on('ready', listener2);

      await sdk.init();

      expect(listener1).toHaveBeenCalled();
      expect(listener2).toHaveBeenCalled();
    });

    it('should remove event listeners', async () => {
      const listener = jest.fn();

      sdk.on('ready', listener);
      sdk.off('ready', listener);

      await sdk.init();

      expect(listener).not.toHaveBeenCalled();
    });

    it('should handle once listeners', async () => {
      const listener = jest.fn();

      sdk.once('ready', listener);

      await sdk.init();
      sdk.emit('ready'); // Emit again

      expect(listener).toHaveBeenCalledTimes(1);
    });
  });

  describe('Error Handling', () => {
    it('should handle WASM loading failure', async () => {
      (WasmLoader.loadWasmSdk as jest.Mock).mockRejectedValue(new Error('WASM load failed'));

      sdk = new SDK({ network: 'testnet' });
      
      await expect(sdk.init()).rejects.toThrow('WASM load failed');
      expect(sdk.isReady()).toBe(false);
    });

    it('should emit error event on initialization failure', async () => {
      (WasmLoader.loadWasmSdk as jest.Mock).mockRejectedValue(new Error('WASM load failed'));

      sdk = new SDK({ network: 'testnet' });
      const errorHandler = jest.fn();
      sdk.on('error', errorHandler);

      try {
        await sdk.init();
      } catch (e) {
        // Expected
      }

      expect(errorHandler).toHaveBeenCalledWith(expect.any(Error));
    });
  });

  describe('Cleanup', () => {
    it('should properly clean up resources on close', async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();

      const wasm = sdk.getWasmSdk();
      sdk.close();

      expect(sdk.isReady()).toBe(false);
      expect(() => sdk.getWasmSdk()).toThrow('SDK not initialized');
    });

    it('should remove all event listeners on close', async () => {
      sdk = new SDK({ network: 'testnet' });
      const listener = jest.fn();
      sdk.on('some-event', listener);

      sdk.close();
      sdk.emit('some-event');

      expect(listener).not.toHaveBeenCalled();
    });
  });

  describe('Integration with Testnet', () => {
    it('should verify all queries use proved mode', async () => {
      sdk = new SDK({ network: 'testnet' });
      await sdk.init();

      // Simulate a balance query
      const fetchOptions = new mockWasmSdk.FetchOptions();
      await mockWasmSdk.fetchIdentityBalance(
        sdk.getWasmSdk(), 
        global.TEST_IDENTITY_ID,
        fetchOptions
      );

      // Verify withProve was called
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);
    });
  });
});