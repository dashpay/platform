import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ShieldedFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let getShieldedPoolStateStub: SinonStub;
  let getShieldedPoolStateWithProofInfoStub: SinonStub;
  let getShieldedEncryptedNotesStub: SinonStub;
  let getShieldedEncryptedNotesWithProofInfoStub: SinonStub;
  let getShieldedAnchorsStub: SinonStub;
  let getShieldedAnchorsWithProofInfoStub: SinonStub;
  let getMostRecentShieldedAnchorStub: SinonStub;
  let getMostRecentShieldedAnchorWithProofInfoStub: SinonStub;
  let getShieldedNullifiersStub: SinonStub;
  let getShieldedNullifiersWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getShieldedPoolStateStub = this.sinon.stub(wasmSdk, 'getShieldedPoolState').resolves(1000n);
    getShieldedPoolStateWithProofInfoStub = this.sinon.stub(wasmSdk, 'getShieldedPoolStateWithProofInfo').resolves('proof');
    getShieldedEncryptedNotesStub = this.sinon.stub(wasmSdk, 'getShieldedEncryptedNotes').resolves([]);
    getShieldedEncryptedNotesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getShieldedEncryptedNotesWithProofInfo').resolves('proof');
    getShieldedAnchorsStub = this.sinon.stub(wasmSdk, 'getShieldedAnchors').resolves([]);
    getShieldedAnchorsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getShieldedAnchorsWithProofInfo').resolves('proof');
    getMostRecentShieldedAnchorStub = this.sinon.stub(wasmSdk, 'getMostRecentShieldedAnchor').resolves(undefined);
    getMostRecentShieldedAnchorWithProofInfoStub = this.sinon.stub(wasmSdk, 'getMostRecentShieldedAnchorWithProofInfo').resolves('proof');
    getShieldedNullifiersStub = this.sinon.stub(wasmSdk, 'getShieldedNullifiers').resolves([]);
    getShieldedNullifiersWithProofInfoStub = this.sinon.stub(wasmSdk, 'getShieldedNullifiersWithProofInfo').resolves('proof');
  });

  describe('poolState()', () => {
    it('should forward to getShieldedPoolState and return its result', async () => {
      const result = await client.shielded.poolState();
      expect(getShieldedPoolStateStub).to.be.calledOnce();
      expect(result).to.equal(1000n);
    });
  });

  describe('poolStateWithProof()', () => {
    it('should forward to getShieldedPoolStateWithProofInfo', async () => {
      await client.shielded.poolStateWithProof();
      expect(getShieldedPoolStateWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('encryptedNotes()', () => {
    it('should forward startIndex/count to getShieldedEncryptedNotes', async () => {
      await client.shielded.encryptedNotes(0n, 10);
      expect(getShieldedEncryptedNotesStub).to.be.calledOnceWithExactly(0n, 10);
    });

    it('should pass through bigint startIndex without truncation', async () => {
      const big = 9_007_199_254_740_993n; // > Number.MAX_SAFE_INTEGER
      await client.shielded.encryptedNotes(big, 5);
      expect(getShieldedEncryptedNotesStub).to.be.calledWith(big, 5);
    });
  });

  describe('encryptedNotesWithProof()', () => {
    it('should forward startIndex/count to getShieldedEncryptedNotesWithProofInfo', async () => {
      await client.shielded.encryptedNotesWithProof(100n, 25);
      expect(getShieldedEncryptedNotesWithProofInfoStub).to.be.calledOnceWithExactly(100n, 25);
    });
  });

  describe('anchors()', () => {
    it('should forward to getShieldedAnchors', async () => {
      await client.shielded.anchors();
      expect(getShieldedAnchorsStub).to.be.calledOnce();
    });
  });

  describe('anchorsWithProof()', () => {
    it('should forward to getShieldedAnchorsWithProofInfo', async () => {
      await client.shielded.anchorsWithProof();
      expect(getShieldedAnchorsWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('mostRecentAnchor()', () => {
    it('should forward to getMostRecentShieldedAnchor', async () => {
      await client.shielded.mostRecentAnchor();
      expect(getMostRecentShieldedAnchorStub).to.be.calledOnce();
    });
  });

  describe('mostRecentAnchorWithProof()', () => {
    it('should forward to getMostRecentShieldedAnchorWithProofInfo', async () => {
      await client.shielded.mostRecentAnchorWithProof();
      expect(getMostRecentShieldedAnchorWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('nullifiers()', () => {
    it('should forward the nullifier array to getShieldedNullifiers', async () => {
      const nullifiers = [new Uint8Array(32).fill(1), new Uint8Array(32).fill(2)];
      await client.shielded.nullifiers(nullifiers);
      expect(getShieldedNullifiersStub).to.be.calledOnceWithExactly(nullifiers);
    });

    it('should accept an empty array', async () => {
      await client.shielded.nullifiers([]);
      expect(getShieldedNullifiersStub).to.be.calledOnceWithExactly([]);
    });
  });

  describe('nullifiersWithProof()', () => {
    it('should forward the nullifier array to getShieldedNullifiersWithProofInfo', async () => {
      const nullifiers = [new Uint8Array(32).fill(7)];
      await client.shielded.nullifiersWithProof(nullifiers);
      expect(getShieldedNullifiersWithProofInfoStub).to.be.calledOnceWithExactly(nullifiers);
    });
  });
});
