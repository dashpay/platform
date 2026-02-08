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
  let getDpnsUsernamesStub: SinonStub;
  let getDpnsUsernameStub: SinonStub;
  let getDpnsUsernamesWithProofInfoStub: SinonStub;
  let getDpnsUsernameWithProofInfoStub: SinonStub;
  let getDpnsUsernameByNameStub: SinonStub;
  let getDpnsUsernameByNameWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    dpnsIsNameAvailableStub = this.sinon.stub(wasmSdk, 'dpnsIsNameAvailable').resolves(true);
    dpnsResolveNameStub = this.sinon.stub(wasmSdk, 'dpnsResolveName').resolves({});
    dpnsRegisterNameStub = this.sinon.stub(wasmSdk, 'dpnsRegisterName').resolves({});
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
