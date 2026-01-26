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
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
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

  it('should await wasm statics for convertToHomographSafe/isValidUsername/isContestedUsername', async () => {
    const out1 = await client.dpns.convertToHomographSafe('abc');
    const out2 = await client.dpns.isValidUsername('abc');
    const out3 = await client.dpns.isContestedUsername('abc');
    expect(out1).to.be.ok();
    expect(out2).to.be.a('boolean');
    expect(out3).to.be.a('boolean');
  });

  it('should forward name resolution and registration correctly', async () => {
    await client.dpns.isNameAvailable('label');
    await client.dpns.resolveName('name');

    // New API uses identity, identityKey, signer instead of identityId/publicKeyId/privateKeyWif
    const mockIdentity = {};
    const mockIdentityKey = {};
    const mockSigner = {};
    await client.dpns.registerName({
      label: 'l',
      identity: mockIdentity,
      identityKey: mockIdentityKey,
      signer: mockSigner,
    });
    await client.dpns.usernames({ identityId: 'i', limit: 2 });
    await client.dpns.username('i');
    await client.dpns.usernamesWithProof({ identityId: 'i', limit: 3 });
    await client.dpns.usernameWithProof('i');
    await client.dpns.getUsernameByName('u.dash');
    await client.dpns.getUsernameByNameWithProof('u.dash');

    expect(dpnsIsNameAvailableStub).to.be.calledOnceWithExactly('label');
    expect(dpnsResolveNameStub).to.be.calledOnceWithExactly('name');
    expect(dpnsRegisterNameStub).to.be.calledOnce();
    expect(getDpnsUsernamesStub).to.be.calledOnceWithExactly({ identityId: 'i', limit: 2 });
    expect(getDpnsUsernameStub).to.be.calledOnceWithExactly('i');
    expect(getDpnsUsernamesWithProofInfoStub).to.be.calledOnceWithExactly({ identityId: 'i', limit: 3 });
    expect(getDpnsUsernameWithProofInfoStub).to.be.calledOnceWithExactly('i');
    expect(getDpnsUsernameByNameStub).to.be.calledOnceWithExactly('u.dash');
    expect(getDpnsUsernameByNameWithProofInfoStub).to.be.calledOnceWithExactly('u.dash');
  });
});
