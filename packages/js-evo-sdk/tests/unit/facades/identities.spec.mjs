import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('IdentitiesFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getIdentity').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityUnproved').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityKeys').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityKeysWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityNonce').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityNonceWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityContractNonce').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityContractNonceWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityBalance').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityBalanceWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesBalances').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesBalancesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityBalanceAndRevision').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityBalanceAndRevisionWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityByPublicKeyHash').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityByPublicKeyHashWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHash').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHashWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesContractKeys').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesContractKeysWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves('ok');
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
    await client.identities.getKeys({
      identityId: 'id',
      keyRequestType: 'specific',
      specificKeyIds: [1, 2],
      searchPurposeMap: { a: 1 },
      limit: 10,
      offset: 2,
    });
    const { args } = wasmSdk.getIdentityKeys.firstCall;
    expect(args[0]).to.equal('id');
    expect(args[1]).to.equal('specific');
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([1, 2]);
    expect(args[3]).to.equal(JSON.stringify({ a: 1 }));
    expect(args[4]).to.equal(10);
    expect(args[5]).to.equal(2);
  });

  it('getKeysWithProof() forwards with Uint32Array', async () => {
    await client.identities.getKeysWithProof({
      identityId: 'id',
      keyRequestType: 'specific',
      specificKeyIds: [5],
      limit: 1,
      offset: 0,
    });
    const { args } = wasmSdk.getIdentityKeysWithProofInfo.firstCall;
    expect(args[0]).to.equal('id');
    expect(args[1]).to.equal('specific');
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([5]);
    expect(args[3]).to.equal(1);
    expect(args[4]).to.equal(0);
  });

  it('nonce helpers forward to wasm', async () => {
    await client.identities.nonce('id');
    await client.identities.nonceWithProof('id');
    expect(wasmSdk.getIdentityNonce).to.be.calledOnceWithExactly('id');
    expect(wasmSdk.getIdentityNonceWithProofInfo).to.be.calledOnceWithExactly('id');
  });

  it('contractNonce helpers forward to wasm', async () => {
    await client.identities.contractNonce('id', 'contract');
    await client.identities.contractNonceWithProof('id', 'contract');
    expect(wasmSdk.getIdentityContractNonce).to.be.calledOnceWithExactly('id', 'contract');
    expect(wasmSdk.getIdentityContractNonceWithProofInfo).to.be.calledOnceWithExactly('id', 'contract');
  });

  it('balance helpers forward to wasm', async () => {
    await client.identities.balance('id');
    await client.identities.balanceWithProof('id');
    await client.identities.balances(['a', 'b']);
    await client.identities.balancesWithProof(['c']);
    expect(wasmSdk.getIdentityBalance).to.be.calledOnceWithExactly('id');
    expect(wasmSdk.getIdentityBalanceWithProofInfo).to.be.calledOnceWithExactly('id');
    expect(wasmSdk.getIdentitiesBalances).to.be.calledOnceWithExactly(['a', 'b']);
    expect(wasmSdk.getIdentitiesBalancesWithProofInfo).to.be.calledOnceWithExactly(['c']);
  });

  it('balanceAndRevision helpers forward to wasm', async () => {
    await client.identities.balanceAndRevision('id');
    await client.identities.balanceAndRevisionWithProof('id');
    expect(wasmSdk.getIdentityBalanceAndRevision).to.be.calledOnceWithExactly('id');
    expect(wasmSdk.getIdentityBalanceAndRevisionWithProofInfo).to.be.calledOnceWithExactly('id');
  });

  it('public key hash lookups forward to wasm', async () => {
    await client.identities.byPublicKeyHash('hash');
    await client.identities.byPublicKeyHashWithProof('hash');
    await client.identities.byNonUniquePublicKeyHash('hash', { startAfter: 'cursor' });
    await client.identities.byNonUniquePublicKeyHashWithProof('hash');
    expect(wasmSdk.getIdentityByPublicKeyHash).to.be.calledOnceWithExactly('hash');
    expect(wasmSdk.getIdentityByPublicKeyHashWithProofInfo).to.be.calledOnceWithExactly('hash');
    expect(wasmSdk.getIdentityByNonUniquePublicKeyHash).to.be.calledOnceWithExactly('hash', 'cursor');
    expect(wasmSdk.getIdentityByNonUniquePublicKeyHashWithProofInfo).to.be.calledOnceWithExactly('hash', null);
  });

  it('contractKeys helpers convert purposes to Uint32Array and forward', async () => {
    await client.identities.contractKeys({ identityIds: ['a'], contractId: 'c', purposes: [1, 2] });
    await client.identities.contractKeysWithProof({ identityIds: ['b'], contractId: 'c' });
    const arrayCall = wasmSdk.getIdentitiesContractKeys.firstCall.args;
    expect(arrayCall[0]).to.deep.equal(['a']);
    expect(arrayCall[1]).to.equal('c');
    expect(arrayCall[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(arrayCall[2])).to.deep.equal([1, 2]);
    expect(wasmSdk.getIdentitiesContractKeysWithProofInfo).to.be.calledOnceWithExactly(['b'], 'c', null);
  });

  it('tokenBalances helpers forward to wasm', async () => {
    await client.identities.tokenBalances('id', ['t1']);
    await client.identities.tokenBalancesWithProof('id', ['t2']);
    expect(wasmSdk.getIdentityTokenBalances).to.be.calledOnceWithExactly('id', ['t1']);
    expect(wasmSdk.getIdentityTokenBalancesWithProofInfo).to.be.calledOnceWithExactly('id', ['t2']);
  });

  it('create() calls wasmSdk.identityCreate with JSON proof and keys', async () => {
    await client.identities.create({
      assetLockProof: { p: true },
      assetLockPrivateKeyWif: 'w',
      publicKeys: [{ k: 1 }],
    });
    expect(wasmSdk.identityCreate).to.be.calledOnce();
    const [proofJson, wif, keysJson] = wasmSdk.identityCreate.firstCall.args;
    expect(proofJson).to.equal(JSON.stringify({ p: true }));
    expect(wif).to.equal('w');
    expect(keysJson).to.equal(JSON.stringify([{ k: 1 }]));
  });

  it('topUp() calls wasmSdk.identityTopUp with JSON proof', async () => {
    await client.identities.topUp({
      identityId: 'id',
      assetLockProof: { p: 1 },
      assetLockPrivateKeyWif: 'w',
    });
    expect(wasmSdk.identityTopUp).to.be.calledOnceWithExactly('id', JSON.stringify({ p: 1 }), 'w');
  });

  it('creditTransfer() converts amount to BigInt', async () => {
    await client.identities.creditTransfer({
      senderId: 's',
      recipientId: 'r',
      amount: '5',
      privateKeyWif: 'w',
      keyId: 3,
    });
    const { args } = wasmSdk.identityCreditTransfer.firstCall;
    expect(args[0]).to.equal('s');
    expect(args[1]).to.equal('r');
    expect(typeof args[2]).to.equal('bigint');
    expect(args[2]).to.equal(BigInt(5));
    expect(args.slice(3)).to.deep.equal(['w', 3]);
  });

  it('creditWithdrawal() converts amount to BigInt and passes coreFeePerByte', async () => {
    await client.identities.creditWithdrawal({
      identityId: 'i',
      toAddress: 'addr',
      amount: 7,
      coreFeePerByte: 2,
      privateKeyWif: 'w',
      keyId: 4,
    });
    const { args } = wasmSdk.identityCreditWithdrawal.firstCall;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal('addr');
    expect(args[2]).to.equal(BigInt(7));
    expect(args[3]).to.equal(2);
    expect(args[4]).to.equal('w');
    expect(args[5]).to.equal(4);
  });

  it('update() passes JSON for keys and Uint32Array for disabled key ids', async () => {
    await client.identities.update({
      identityId: 'i',
      addPublicKeys: [{ k: 1 }],
      disablePublicKeyIds: [10, 20],
      privateKeyWif: 'w',
    });
    const { args } = wasmSdk.identityUpdate.firstCall;
    expect(args[0]).to.equal('i');
    expect(args[1]).to.equal(JSON.stringify([{ k: 1 }]));
    expect(args[2]).to.be.instanceOf(Uint32Array);
    expect(Array.from(args[2])).to.deep.equal([10, 20]);
    expect(args[3]).to.equal('w');
  });
});
