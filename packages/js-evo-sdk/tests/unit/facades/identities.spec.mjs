import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import sinon from 'sinon';

const isBrowser = typeof window !== 'undefined';

describe('IdentitiesFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('fetch() and fetchWithProof() forward to free functions', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.identities.fetch('id');
    await sdk.identities.fetchWithProof('id2');
    const calls = wasmStubModule.__getCalls();
    expect(calls[0].called).to.equal('identity_fetch');
    expect(calls[0].args).to.deep.equal([raw, 'id']);
    expect(calls[1].called).to.equal('identity_fetch_with_proof_info');
    expect(calls[1].args).to.deep.equal([raw, 'id2']);
  });

  it('fetchUnproved() forwards to identity_fetch_unproved', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.identities.fetchUnproved('id');
    const last = wasmStubModule.__getCalls().pop();
    expect(last.called).to.equal('identity_fetch_unproved');
    expect(last.args).to.deep.equal([raw, 'id']);
  });

  it('getKeys() forwards with Uint32Array and JSON mapping', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.identities.getKeys({ identityId: 'id', keyRequestType: 'specific', specificKeyIds: [1, 2], searchPurposeMap: { a: 1 }, limit: 10, offset: 2 });
    const last = wasmStubModule.__getCalls().pop();
    expect(last.called).to.equal('get_identity_keys');
    expect(last.args[0]).to.equal(raw);
    expect(last.args[1]).to.equal('id');
    expect(last.args[2]).to.equal('specific');
    expect(last.args[3]).to.be.instanceOf(Uint32Array);
    expect(Array.from(last.args[3])).to.deep.equal([1, 2]);
    expect(last.args[4]).to.equal(JSON.stringify({ a: 1 }));
    expect(last.args[5]).to.equal(10);
    expect(last.args[6]).to.equal(2);
  });

  it('create() calls wasmSdk.identityCreate with JSON proof and keys', async () => {
    const wasm = { identityCreate: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.identities.create({ assetLockProof: { p: true }, assetLockPrivateKeyWif: 'w', publicKeys: [{ k: 1 }] });
    sinon.assert.calledOnce(wasm.identityCreate);
    const [proofJson, wif, keysJson] = wasm.identityCreate.firstCall.args;
    expect(proofJson).to.equal(JSON.stringify({ p: true }));
    expect(wif).to.equal('w');
    expect(keysJson).to.equal(JSON.stringify([{ k: 1 }]));
  });

  it('topUp() calls wasmSdk.identityTopUp with JSON proof', async () => {
    const wasm = { identityTopUp: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.identities.topUp({ identityId: 'id', assetLockProof: { p: 1 }, assetLockPrivateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.identityTopUp, 'id', JSON.stringify({ p: 1 }), 'w');
  });

  it('creditTransfer() converts amount to BigInt', async () => {
    const wasm = { identityCreditTransfer: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.identities.creditTransfer({ senderId: 's', recipientId: 'r', amount: '5', privateKeyWif: 'w', keyId: 3 });
    const args = wasm.identityCreditTransfer.firstCall.args;
    expect(args[0]).to.equal('s');
    expect(args[1]).to.equal('r');
    expect(typeof args[2]).to.equal('bigint');
    expect(args[2]).to.equal(5n);
    expect(args.slice(3)).to.deep.equal(['w', 3]);
  });

  it('creditWithdrawal() converts amount to BigInt and passes coreFeePerByte', async () => {
    const wasm = { identityCreditWithdrawal: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.identities.creditWithdrawal({ identityId: 'i', toAddress: 'addr', amount: 7, coreFeePerByte: 2, privateKeyWif: 'w', keyId: 4 });
    const args = wasm.identityCreditWithdrawal.firstCall.args;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal('addr');
    expect(args[2]).to.equal(7n);
    expect(args[3]).to.equal(2);
    expect(args[4]).to.equal('w');
    expect(args[5]).to.equal(4);
  });

  it('update() passes JSON for keys and Uint32Array for disabled key ids', async () => {
    const wasm = { identityUpdate: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.identities.update({ identityId: 'i', addPublicKeys: [{ k: 1 }], disablePublicKeyIds: [10, 20], privateKeyWif: 'w' });
    const args = wasm.identityUpdate.firstCall.args;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal(JSON.stringify([{ k: 1 }]));
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([10, 20]);
    expect(args[3]).to.equal('w');
  });
});

