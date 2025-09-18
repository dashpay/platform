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

  it('convertToHomographSafe/isValidUsername/isContestedUsername use class statics', () => {
    const out1 = wasmSDKPackage.WasmSdk.dpnsConvertToHomographSafe('abc');
    const out2 = wasmSDKPackage.WasmSdk.dpnsIsValidUsername('abc');
    const out3 = wasmSDKPackage.WasmSdk.dpnsIsContestedUsername('abc');
    expect(out1).to.be.ok();
    expect(typeof out2).to.not.equal('undefined');
    expect(typeof out3).to.not.equal('undefined');
  });

  it('name resolution and registration forward correctly', async () => {
    await client.dpns.isNameAvailable('label');
    await client.dpns.resolveName('name');
    await client.dpns.registerName({
      label: 'l', identityId: 'i', publicKeyId: 1, privateKeyWif: 'w',
    });
    await client.dpns.usernames('i', { limit: 2 });
    await client.dpns.username('i');
    await client.dpns.usernamesWithProof('i', { limit: 3 });
    await client.dpns.usernameWithProof('i');
    await client.dpns.getUsernameByName('u');
    await client.dpns.getUsernameByNameWithProof('u');

    expect(wasmSdk.dpnsIsNameAvailable).to.be.calledOnceWithExactly('label');
    expect(wasmSdk.dpnsResolveName).to.be.calledOnceWithExactly('name');
    expect(wasmSdk.dpnsRegisterName).to.be.calledOnce();
    expect(wasmSdk.getDpnsUsernames).to.be.calledOnceWithExactly('i', 2);
    expect(wasmSdk.getDpnsUsername).to.be.calledOnceWithExactly('i');
    expect(wasmSdk.getDpnsUsernamesWithProofInfo).to.be.calledOnceWithExactly('i', 3);
    expect(wasmSdk.getDpnsUsernameWithProofInfo).to.be.calledOnceWithExactly('i');
    expect(wasmSdk.getDpnsUsernameByName).to.be.calledOnceWithExactly('u');
    expect(wasmSdk.getDpnsUsernameByNameWithProofInfo).to.be.calledOnceWithExactly('u');
  });
});
