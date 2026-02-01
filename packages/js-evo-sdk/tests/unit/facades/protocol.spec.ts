import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ProtocolFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let getProtocolVersionUpgradeStateStub: SinonStub;
  let getProtocolVersionUpgradeStateWithProofInfoStub: SinonStub;
  let getProtocolVersionUpgradeVoteStatusStub: SinonStub;
  let getProtocolVersionUpgradeVoteStatusWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getProtocolVersionUpgradeStateStub = this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeState').resolves('ok');
    getProtocolVersionUpgradeStateWithProofInfoStub = this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeStateWithProofInfo').resolves('ok');
    getProtocolVersionUpgradeVoteStatusStub = this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeVoteStatus').resolves('ok');
    getProtocolVersionUpgradeVoteStatusWithProofInfoStub = this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeVoteStatusWithProofInfo').resolves('ok');
  });

  describe('versionUpgradeState()', () => {
    it('should forward to getProtocolVersionUpgradeState', async () => {
      await client.protocol.versionUpgradeState();
      expect(getProtocolVersionUpgradeStateStub).to.be.calledOnce();
    });
  });

  describe('versionUpgradeStateWithProof()', () => {
    it('should forward to getProtocolVersionUpgradeStateWithProofInfo', async () => {
      await client.protocol.versionUpgradeStateWithProof();
      expect(getProtocolVersionUpgradeStateWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('versionUpgradeVoteStatus()', () => {
    it('should forward with positional args', async () => {
      await client.protocol.versionUpgradeVoteStatus('h', 5);
      expect(getProtocolVersionUpgradeVoteStatusStub).to.be.calledOnceWithExactly('h', 5);
    });

    it('should accept Uint8Array and positional args', async () => {
      const bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
      await client.protocol.versionUpgradeVoteStatus(bytes, 2);
      expect(getProtocolVersionUpgradeVoteStatusStub).to.be.calledWith(bytes, 2);
    });
  });

  describe('versionUpgradeVoteStatusWithProof()', () => {
    it('should forward with positional args', async () => {
      await client.protocol.versionUpgradeVoteStatusWithProof('g', 3);
      expect(getProtocolVersionUpgradeVoteStatusWithProofInfoStub).to.be.calledOnceWithExactly('g', 3);
    });

    it('should accept Uint8Array and positional args', async () => {
      const bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
      await client.protocol.versionUpgradeVoteStatusWithProof(bytes, 4);
      expect(getProtocolVersionUpgradeVoteStatusWithProofInfoStub).to.be.calledWith(bytes, 4);
    });
  });
});
