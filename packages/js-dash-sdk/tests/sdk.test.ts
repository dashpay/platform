import { createSDK, SDK, CentralizedProvider } from '../src';

describe('SDK', () => {
  describe('initialization', () => {
    it('should create SDK instance with default options', () => {
      const sdk = createSDK();
      expect(sdk).toBeInstanceOf(SDK);
      expect(sdk.getNetwork().type).toBe('testnet');
    });

    it('should create SDK instance with custom network', () => {
      const sdk = createSDK({ network: 'mainnet' });
      expect(sdk.getNetwork().type).toBe('mainnet');
    });

    it('should create SDK instance with custom provider', () => {
      const provider = new CentralizedProvider({ 
        url: 'https://custom.provider.com' 
      });
      const sdk = createSDK({ contextProvider: provider });
      expect(sdk.getContextProvider()).toBe(provider);
    });

    it('should throw error when accessing WASM before initialization', () => {
      const sdk = createSDK();
      expect(() => sdk.getWasmSdk()).toThrow('SDK not initialized');
    });
  });

  describe('app management', () => {
    let sdk: SDK;

    beforeEach(() => {
      sdk = createSDK();
    });

    it('should register and retrieve apps', () => {
      const appDef = { contractId: 'test-contract-id' };
      sdk.registerApp('testapp', appDef);
      
      expect(sdk.hasApp('testapp')).toBe(true);
      expect(sdk.getApp('testapp')).toEqual(appDef);
    });

    it('should return undefined for non-existent app', () => {
      expect(sdk.getApp('nonexistent')).toBeUndefined();
      expect(sdk.hasApp('nonexistent')).toBe(false);
    });

    it('should return all registered apps', () => {
      sdk.registerApp('app1', { contractId: 'id1' });
      sdk.registerApp('app2', { contractId: 'id2' });
      
      const apps = sdk.getApps();
      expect(Object.keys(apps)).toHaveLength(2);
      expect(apps.app1.contractId).toBe('id1');
      expect(apps.app2.contractId).toBe('id2');
    });

    it('should handle duplicate app registration', () => {
      const appDef1 = { contractId: 'id1' };
      const appDef2 = { contractId: 'id2' };
      
      sdk.registerApp('testapp', appDef1);
      expect(sdk.getApp('testapp')).toEqual(appDef1);
      
      // Register same app name with different definition
      sdk.registerApp('testapp', appDef2);
      expect(sdk.getApp('testapp')).toEqual(appDef2);
      
      // Should have replaced, not added
      const apps = sdk.getApps();
      expect(Object.keys(apps)).toHaveLength(1);
    });
  });

  describe('event emitter', () => {
    it('should emit app:registered event', (done) => {
      const sdk = createSDK();
      const appDef = { contractId: 'test-id' };
      
      sdk.on('app:registered', (event) => {
        expect(event.name).toBe('testapp');
        expect(event.definition).toEqual(appDef);
        done();
      });
      
      sdk.registerApp('testapp', appDef);
    });
  });

  describe('cleanup', () => {
    it('should clean up properly', () => {
      const sdk = createSDK();
      sdk.on('test', () => {});
      
      expect(sdk.listenerCount('test')).toBe(1);
      
      sdk.destroy();
      
      expect(sdk.listenerCount('test')).toBe(0);
      expect(sdk.isInitialized()).toBe(false);
    });
  });
});