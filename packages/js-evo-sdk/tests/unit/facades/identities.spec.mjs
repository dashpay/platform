import { EvoSDK } from '../../../dist/sdk.js';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';

describe('IdentitiesFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getIdentity').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityUnproved').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityKeys').resolves('ok');
    this.sinon.stub(wasmSdk, 'identityCreate').resolves('ok');
    this.sinon.stub(wasmSdk, 'identityTopUp').resolves('ok');
    this.sinon.stub(wasmSdk, 'identityCreditTransfer').resolves('ok');
    this.sinon.stub(wasmSdk, 'identityCreditWithdrawal').resolves('ok');
    this.sinon.stub(wasmSdk, 'identityUpdate').resolves('ok');
  });

  it('fetch() and fetchWithProof() forward to instance methods', async () => {
    await client.identities.fetch('id');
    await client.identities.fetchWithProof('id2');
    expect(wasmSdk.getIdentity).to.be.calledOnceWithExactly('id');
    expect(wasmSdk.getIdentityWithProofInfo).to.be.calledOnceWithExactly('id2');
  });

  it('fetchUnproved() forwards to getIdentityUnproved', async () => {
    await client.identities.fetchUnproved('id');
    expect(wasmSdk.getIdentityUnproved).to.be.calledOnceWithExactly('id');
  });

  it('getKeys() forwards with Uint32Array and JSON mapping', async () => {
    await client.identities.getKeys({ identityId: 'id', keyRequestType: 'specific', specificKeyIds: [1, 2], searchPurposeMap: { a: 1 }, limit: 10, offset: 2 });
    const args = wasmSdk.getIdentityKeys.firstCall.args;
    expect(args[0]).to.equal('id');
    expect(args[1]).to.equal('specific');
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([1, 2]);
    expect(args[3]).to.equal(JSON.stringify({ a: 1 }));
    expect(args[4]).to.equal(10);
    expect(args[5]).to.equal(2);
  });

  it('create() calls wasmSdk.identityCreate with JSON proof and keys', async () => {
    await client.identities.create({ assetLockProof: { p: true }, assetLockPrivateKeyWif: 'w', publicKeys: [{ k: 1 }] });
    expect(wasmSdk.identityCreate).to.be.calledOnce();
    const [proofJson, wif, keysJson] = wasmSdk.identityCreate.firstCall.args;
    expect(proofJson).to.equal(JSON.stringify({ p: true }));
    expect(wif).to.equal('w');
    expect(keysJson).to.equal(JSON.stringify([{ k: 1 }]));
  });

  it('topUp() calls wasmSdk.identityTopUp with JSON proof', async () => {
    await client.identities.topUp({ identityId: 'id', assetLockProof: { p: 1 }, assetLockPrivateKeyWif: 'w' });
    expect(wasmSdk.identityTopUp).to.be.calledOnceWithExactly('id', JSON.stringify({ p: 1 }), 'w');
  });

  it('creditTransfer() converts amount to BigInt', async () => {
    await client.identities.creditTransfer({ senderId: 's', recipientId: 'r', amount: '5', privateKeyWif: 'w', keyId: 3 });
    const args = wasmSdk.identityCreditTransfer.firstCall.args;
    expect(args[0]).to.equal('s');
    expect(args[1]).to.equal('r');
    expect(typeof args[2]).to.equal('bigint');
    expect(args[2]).to.equal(5n);
    expect(args.slice(3)).to.deep.equal(['w', 3]);
  });

  it('creditWithdrawal() converts amount to BigInt and passes coreFeePerByte', async () => {
    await client.identities.creditWithdrawal({ identityId: 'i', toAddress: 'addr', amount: 7, coreFeePerByte: 2, privateKeyWif: 'w', keyId: 4 });
    const args = wasmSdk.identityCreditWithdrawal.firstCall.args;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal('addr');
    expect(args[2]).to.equal(7n);
    expect(args[3]).to.equal(2);
    expect(args[4]).to.equal('w');
    expect(args[5]).to.equal(4);
  });

  it('update() passes JSON for keys and Uint32Array for disabled key ids', async () => {
    await client.identities.update({ identityId: 'i', addPublicKeys: [{ k: 1 }], disablePublicKeyIds: [10, 20], privateKeyWif: 'w' });
    const args = wasmSdk.identityUpdate.firstCall.args;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal(JSON.stringify([{ k: 1 }]));
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([10, 20]);
    expect(args[3]).to.equal('w');
  });
});
