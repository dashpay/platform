import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../dist/sdk.js';

// Builds a real SDK through the real wasm builder. Building dials nothing, and
// the verified context proves keys only when a proof needs one.
describe('EvoSDK with verified: true (real wasm builder)', () => {
  before(async () => {
    await init();
  });

  it('should connect and report the embedded checkpoint as the latest verified state', async () => {
    const sdk = EvoSDK.testnetVerified();

    await sdk.connect();

    const embedded = wasmSDKPackage.WasmVerifiedContext.testnet();
    expect(sdk.isConnected).to.equal(true);
    expect(sdk.latestVerifiedState()).to.deep.equal(embedded.checkpoint());
    embedded.free();
  });

  it('should start from a saved state', async () => {
    const saved = JSON.parse(JSON.stringify(wasmSDKPackage.WasmVerifiedContext.testnet().checkpoint()));
    const sdk = EvoSDK.testnetVerified({ checkpoint: saved, quorumRelays: ['https://relay.test'] });

    await sdk.connect();

    expect(sdk.latestVerifiedState()).to.deep.equal(saved);
  });

  it('should refuse to connect with proofs turned off', async () => {
    const sdk = EvoSDK.testnetVerified({ proofs: false });
    let thrown: unknown;
    try {
      await sdk.connect();
    } catch (error) {
      thrown = error;
    }
    expect((thrown as { message?: string })?.message).to.match(/withProofs/);
    expect(sdk.isConnected).to.equal(false);
  });

  it('should surface an invalid relay from the wasm side on connect', async () => {
    const sdk = EvoSDK.testnetVerified({ quorumRelays: ['http://relay.test'] });
    let thrown: unknown;
    try {
      await sdk.connect();
    } catch (error) {
      thrown = error;
    }
    expect((thrown as { message?: string })?.message).to.match(/https/);
    expect(sdk.isConnected).to.equal(false);
  });
});
