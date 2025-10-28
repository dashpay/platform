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

  describe('registerName validation', () => {
    it('should throw error when publicKeyId is not provided', async () => {
      try {
        await client.dpns.registerName({
          label: 'test',
          identityId: 'someId',
          privateKeyWif: 'someKey',
          // publicKeyId intentionally omitted
        });
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error.message).to.include('publicKeyId is required');
        expect(error.message).to.include('CRITICAL or HIGH security level');
      }
    });

    it('should throw error when publicKeyId is undefined', async () => {
      try {
        await client.dpns.registerName({
          label: 'test',
          identityId: 'someId',
          publicKeyId: undefined,
          privateKeyWif: 'someKey',
        });
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error.message).to.include('publicKeyId is required');
        expect(error.message).to.include('CRITICAL or HIGH security level');
      }
    });

    it('should throw error when publicKeyId is null', async () => {
      try {
        await client.dpns.registerName({
          label: 'test',
          identityId: 'someId',
          publicKeyId: null,
          privateKeyWif: 'someKey',
        });
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error.message).to.include('publicKeyId is required');
      }
    });

    it('should throw error when publicKeyId is negative', async () => {
      try {
        await client.dpns.registerName({
          label: 'test',
          identityId: 'someId',
          publicKeyId: -1,
          privateKeyWif: 'someKey',
        });
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error.message).to.include('must be a non-negative number');
        expect(error.message).to.include('got: -1');
      }
    });

    it('should throw error when publicKeyId is not a number', async () => {
      try {
        await client.dpns.registerName({
          label: 'test',
          identityId: 'someId',
          publicKeyId: '1',
          privateKeyWif: 'someKey',
        });
        expect.fail('Should have thrown an error');
      } catch (error) {
        expect(error.message).to.include('must be a non-negative number');
      }
    });

    it('should accept valid publicKeyId', async () => {
      await client.dpns.registerName({
        label: 'test',
        identityId: 'someId',
        publicKeyId: 1,
        privateKeyWif: 'someKey',
      });
      expect(wasmSdk.dpnsRegisterName).to.be.calledOnce();
    });
  });
});
