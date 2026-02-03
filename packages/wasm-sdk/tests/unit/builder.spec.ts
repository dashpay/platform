import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('WasmSdkBuilder', () => {
  before(async () => {
    await init();
  });

  describe('static methods', () => {
    it('should expose WasmSdkBuilder as a function', () => {
      expect(sdk.WasmSdkBuilder).to.be.a('function');
    });

    it('should expose getLatestVersionNumber method', () => {
      expect(sdk.WasmSdkBuilder.getLatestVersionNumber).to.be.a('function');
    });

    it('should expose mainnet method', () => {
      expect(sdk.WasmSdkBuilder.mainnet).to.be.a('function');
    });

    it('should expose testnet method', () => {
      expect(sdk.WasmSdkBuilder.testnet).to.be.a('function');
    });

    it('should expose local method', () => {
      expect(sdk.WasmSdkBuilder.local).to.be.a('function');
    });
  });

  describe('testnet()', () => {
    it('should create a builder instance', () => {
      const builder = sdk.WasmSdkBuilder.testnet();
      expect(builder).to.be.ok();
    });
  });

  describe('withVersion()', () => {
    it('should set version and return a new builder', async () => {
      let builder = sdk.WasmSdkBuilder.testnet();
      builder = builder.withVersion(1);
      const built = await builder.build();
      expect(built).to.be.ok();
      built.free();
    });
  });

  describe('withSettings()', () => {
    it('should apply custom settings (timeouts, retries, ban flag)', async () => {
      let builder = sdk.WasmSdkBuilder.testnet();
      builder = builder.withSettings(5000, 10000, 3, true);
      const built = await builder.build();
      expect(built).to.be.ok();
      built.free();
    });
  });

  describe('build()', () => {
    it('should build a WasmSdk instance', async () => {
      const builder = sdk.WasmSdkBuilder.testnet();
      const built = await builder.build();
      expect(built).to.be.ok();
      built.free();
    });
  });
});
