import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DPNSFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'dpnsIsNameAvailable').resolves(true);
    this.sinon.stub(wasmSdk, 'dpnsResolveName').resolves({});
    this.sinon.stub(wasmSdk, 'dpnsRegisterName').resolves({});
    this.sinon.stub(wasmSdk, 'getDpnsUsernames').resolves([]);
    this.sinon.stub(wasmSdk, 'getDpnsUsername').resolves({});
    this.sinon.stub(wasmSdk, 'getDpnsUsernamesWithProofInfo').resolves({});
    this.sinon.stub(wasmSdk, 'getDpnsUsernameWithProofInfo').resolves({});
    this.sinon.stub(wasmSdk, 'getDpnsUsernameByName').resolves({});
    this.sinon.stub(wasmSdk, 'getDpnsUsernameByNameWithProofInfo').resolves({});
  });

  it('convertToHomographSafe/isValidUsername/isContestedUsername await wasm statics', async () => {
    const out1 = await client.dpns.convertToHomographSafe('abc');
    const out2 = await client.dpns.isValidUsername('abc');
    const out3 = await client.dpns.isContestedUsername('abc');
    expect(out1).to.be.ok();
    expect(out2).to.be.a('boolean');
    expect(out3).to.be.a('boolean');
  });

  it('name resolution and registration forward correctly', async () => {
    await client.dpns.isNameAvailable('label');
    await client.dpns.resolveName('name');

    // New API uses identity, identityKey, and signer instead of identityId/publicKeyId/privateKeyWif
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

    expect(wasmSdk.dpnsIsNameAvailable).to.be.calledOnceWithExactly('label');
    expect(wasmSdk.dpnsResolveName).to.be.calledOnceWithExactly('name');
    expect(wasmSdk.dpnsRegisterName).to.be.calledOnce();
    expect(wasmSdk.getDpnsUsernames).to.be.calledOnceWithExactly({ identityId: 'i', limit: 2 });
    expect(wasmSdk.getDpnsUsername).to.be.calledOnceWithExactly('i');
    expect(wasmSdk.getDpnsUsernamesWithProofInfo).to.be.calledOnceWithExactly({ identityId: 'i', limit: 3 });
    expect(wasmSdk.getDpnsUsernameWithProofInfo).to.be.calledOnceWithExactly('i');
    expect(wasmSdk.getDpnsUsernameByName).to.be.calledOnceWithExactly('u.dash');
    expect(wasmSdk.getDpnsUsernameByNameWithProofInfo).to.be.calledOnceWithExactly('u.dash');
  });

});
