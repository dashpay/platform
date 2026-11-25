import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DPNSFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let dpnsIsNameAvailableStub: SinonStub;
  let dpnsResolveNameStub: SinonStub;
  let dpnsRegisterNameStub: SinonStub;
  let dpnsIsNameTombstonedStub: SinonStub;
  let dpnsDeleteNameStub: SinonStub;
  let dpnsTombstoneNameStub: SinonStub;
  let getDpnsUsernamesStub: SinonStub;
  let getDpnsUsernameStub: SinonStub;
  let getDpnsUsernamesWithProofInfoStub: SinonStub;
  let getDpnsUsernameWithProofInfoStub: SinonStub;
  let getDpnsUsernameByNameStub: SinonStub;
  let getDpnsUsernameByNameWithProofInfoStub: SinonStub;

  // What the wasm stubs resolve to: the facade hands them back untouched
  const deletedDocumentId = {} as wasmSDKPackage.Identifier;
  const tombstone = {} as wasmSDKPackage.Document;

  // Stand-ins for the wasm objects of the delete and tombstone options: the stubs never read them
  const identity = {} as wasmSDKPackage.Identity;
  const identityKey = {} as wasmSDKPackage.IdentityPublicKey;
  const signer = {} as wasmSDKPackage.IdentitySigner;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    dpnsIsNameAvailableStub = this.sinon.stub(wasmSdk, 'dpnsIsNameAvailable').resolves(true);
    dpnsResolveNameStub = this.sinon.stub(wasmSdk, 'dpnsResolveName').resolves({});
    dpnsRegisterNameStub = this.sinon.stub(wasmSdk, 'dpnsRegisterName').resolves({});
    dpnsIsNameTombstonedStub = this.sinon.stub(wasmSdk, 'dpnsIsNameTombstoned').resolves(true);
    dpnsDeleteNameStub = this.sinon.stub(wasmSdk, 'dpnsDeleteName').resolves(deletedDocumentId);
    dpnsTombstoneNameStub = this.sinon.stub(wasmSdk, 'dpnsTombstoneName').resolves(tombstone);
    getDpnsUsernamesStub = this.sinon.stub(wasmSdk, 'getDpnsUsernames').resolves([]);
    getDpnsUsernameStub = this.sinon.stub(wasmSdk, 'getDpnsUsername').resolves({});
    getDpnsUsernamesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDpnsUsernamesWithProofInfo').resolves({});
    getDpnsUsernameWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDpnsUsernameWithProofInfo').resolves({});
    getDpnsUsernameByNameStub = this.sinon.stub(wasmSdk, 'getDpnsUsernameByName').resolves({});
    getDpnsUsernameByNameWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDpnsUsernameByNameWithProofInfo').resolves({});
  });

  describe('convertToHomographSafe()', () => {
    it('should await wasm statics and return a result', async () => {
      const out = await client.dpns.convertToHomographSafe('abc');
      expect(out).to.be.ok();
    });
  });

  describe('isValidUsername()', () => {
    it('should return a boolean', async () => {
      const out = await client.dpns.isValidUsername('abc');
      expect(out).to.be.a('boolean');
    });
  });

  describe('isContestedUsername()', () => {
    it('should return a boolean', async () => {
      const out = await client.dpns.isContestedUsername('abc');
      expect(out).to.be.a('boolean');
    });
  });

  describe('isNameAvailable()', () => {
    it('should forward label to wasm', async () => {
      await client.dpns.isNameAvailable('label');
      expect(dpnsIsNameAvailableStub).to.be.calledOnceWithExactly('label');
    });
  });

  describe('resolveName()', () => {
    it('should forward name to wasm', async () => {
      await client.dpns.resolveName('name');
      expect(dpnsResolveNameStub).to.be.calledOnceWithExactly('name');
    });
  });

  describe('registerName()', () => {
    it('should forward registration options to wasm', async () => {
      const mockIdentity = {};
      const mockIdentityKey = {};
      const mockSigner = {};
      await client.dpns.registerName({
        label: 'l',
        identity: mockIdentity,
        identityKey: mockIdentityKey,
        signer: mockSigner,
      });
      expect(dpnsRegisterNameStub).to.be.calledOnce();
    });
  });

  describe('isNameTombstoned()', () => {
    it('should forward name to wasm and return its answer', async () => {
      const tombstoned = await client.dpns.isNameTombstoned('alice.dash');
      expect(dpnsIsNameTombstonedStub).to.be.calledOnceWithExactly('alice.dash');
      expect(tombstoned).to.equal(true);
    });
  });

  describe('deleteName()', () => {
    it('should forward delete options to wasm and return the deleted document id', async () => {
      const options = { name: 'a-name-longer-than-nineteen', identityKey, signer };
      const deleted = await client.dpns.deleteName(options);
      expect(dpnsDeleteNameStub).to.be.calledOnceWithExactly(options);
      expect(deleted).to.equal(deletedDocumentId);
    });

    it('should reject when wasm refuses the delete', async () => {
      const refusal = new Error('the DPNS name a11ce.dash is contested: it can not be deleted, only tombstoned');
      dpnsDeleteNameStub.rejects(refusal);
      await expect(client.dpns.deleteName({ name: 'alice', identityKey, signer }))
        .to.be.rejectedWith(refusal);
      expect(dpnsTombstoneNameStub).to.not.be.called();
    });
  });

  describe('tombstoneName()', () => {
    it('should forward tombstone options to wasm and return the tombstone document', async () => {
      const options = {
        name: 'alice', identity, identityKey, signer,
      };
      const stored = await client.dpns.tombstoneName(options);
      expect(dpnsTombstoneNameStub).to.be.calledOnceWithExactly(options);
      expect(stored).to.equal(tombstone);
      expect(dpnsDeleteNameStub).to.not.be.called();
    });
  });

  describe('usernames()', () => {
    it('should forward query to wasm', async () => {
      await client.dpns.usernames({ identityId: 'i', limit: 2 });
      expect(getDpnsUsernamesStub).to.be.calledOnceWithExactly({ identityId: 'i', limit: 2 });
    });
  });

  describe('username()', () => {
    it('should forward identity ID to wasm', async () => {
      await client.dpns.username('i');
      expect(getDpnsUsernameStub).to.be.calledOnceWithExactly('i');
    });
  });

  describe('usernamesWithProof()', () => {
    it('should forward query to wasm', async () => {
      await client.dpns.usernamesWithProof({ identityId: 'i', limit: 3 });
      expect(getDpnsUsernamesWithProofInfoStub).to.be.calledOnceWithExactly({ identityId: 'i', limit: 3 });
    });
  });

  describe('usernameWithProof()', () => {
    it('should forward identity ID to wasm', async () => {
      await client.dpns.usernameWithProof('i');
      expect(getDpnsUsernameWithProofInfoStub).to.be.calledOnceWithExactly('i');
    });
  });

  describe('getUsernameByName()', () => {
    it('should forward name to wasm', async () => {
      await client.dpns.getUsernameByName('u.dash');
      expect(getDpnsUsernameByNameStub).to.be.calledOnceWithExactly('u.dash');
    });
  });

  describe('getUsernameByNameWithProof()', () => {
    it('should forward name to wasm', async () => {
      await client.dpns.getUsernameByNameWithProof('u.dash');
      expect(getDpnsUsernameByNameWithProofInfoStub).to.be.calledOnceWithExactly('u.dash');
    });
  });
});
