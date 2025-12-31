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
    this.sinon.stub(wasmSdk, 'identityCreate').resolves();
    this.sinon.stub(wasmSdk, 'identityTopUp').resolves(BigInt(1000));
    this.sinon.stub(wasmSdk, 'identityCreditTransfer').resolves({ senderBalance: BigInt(500), recipientBalance: BigInt(500) });
    this.sinon.stub(wasmSdk, 'identityCreditWithdrawal').resolves(BigInt(800));
    this.sinon.stub(wasmSdk, 'identityUpdate').resolves();
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

  it('getKeys() forwards IdentityKeysQuery', async () => {
    const query = {
      identityId: 'id',
      request: {
        type: 'specific',
        specificKeyIds: [1, 2],
      },
      limit: 10,
      offset: 2,
    };
    await client.identities.getKeys(query);
    expect(wasmSdk.getIdentityKeys).to.be.calledOnceWithExactly(query);
  });

  it('getKeysWithProof() forwards IdentityKeysQuery', async () => {
    const query = {
      identityId: 'id',
      request: { type: 'all' },
    };
    await client.identities.getKeysWithProof(query);
    expect(wasmSdk.getIdentityKeysWithProofInfo).to.be.calledOnceWithExactly(query);
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
    await client.identities.byNonUniquePublicKeyHash('hash', 'cursor');
    await client.identities.byNonUniquePublicKeyHashWithProof('hash');
    expect(wasmSdk.getIdentityByPublicKeyHash).to.be.calledOnceWithExactly('hash');
    expect(wasmSdk.getIdentityByPublicKeyHashWithProofInfo).to.be.calledOnceWithExactly('hash');
    expect(wasmSdk.getIdentityByNonUniquePublicKeyHash).to.be.calledOnceWithExactly('hash', 'cursor');
    expect(wasmSdk.getIdentityByNonUniquePublicKeyHashWithProofInfo).to.be.calledOnceWithExactly('hash', undefined);
  });

  it('contractKeys helpers forward query object', async () => {
    const query = { identityIds: ['a'], contractId: 'c', purposes: [1, 2] };
    await client.identities.contractKeys(query);
    const proofQuery = { identityIds: ['b'], contractId: 'c' };
    await client.identities.contractKeysWithProof(proofQuery);
    expect(wasmSdk.getIdentitiesContractKeys).to.be.calledOnceWithExactly(query);
    expect(wasmSdk.getIdentitiesContractKeysWithProofInfo).to.be.calledOnceWithExactly(proofQuery);
  });

  it('tokenBalances helpers forward to wasm', async () => {
    await client.identities.tokenBalances('id', ['t1']);
    await client.identities.tokenBalancesWithProof('id', ['t2']);
    expect(wasmSdk.getIdentityTokenBalances).to.be.calledOnceWithExactly('id', ['t1']);
    expect(wasmSdk.getIdentityTokenBalancesWithProofInfo).to.be.calledOnceWithExactly('id', ['t2']);
  });

  it('create() calls wasmSdk.identityCreate with options', async () => {
    const options = {
      assetLockProof: { p: true },
      signer: { assetLockPrivateKeyWif: 'w' },
      publicKeys: [{ k: 1 }],
    };
    await client.identities.create(options);
    expect(wasmSdk.identityCreate).to.be.calledOnceWithExactly(options);
  });

  it('topUp() calls wasmSdk.identityTopUp with options', async () => {
    const options = {
      identityId: 'id',
      assetLockProof: { p: 1 },
      signer: { assetLockPrivateKeyWif: 'w' },
    };
    await client.identities.topUp(options);
    expect(wasmSdk.identityTopUp).to.be.calledOnceWithExactly(options);
  });

  it('creditTransfer() calls wasmSdk.identityCreditTransfer with options', async () => {
    const options = {
      senderId: 's',
      recipientId: 'r',
      amount: BigInt(5),
      signer: { privateKeyWif: 'w', keyId: 3 },
    };
    await client.identities.creditTransfer(options);
    expect(wasmSdk.identityCreditTransfer).to.be.calledOnceWithExactly(options);
  });

  it('creditWithdrawal() calls wasmSdk.identityCreditWithdrawal with options', async () => {
    const options = {
      identityId: 'i',
      toAddress: 'addr',
      amount: BigInt(7),
      coreFeePerByte: 2,
      signer: { privateKeyWif: 'w', keyId: 4 },
    };
    await client.identities.creditWithdrawal(options);
    expect(wasmSdk.identityCreditWithdrawal).to.be.calledOnceWithExactly(options);
  });

  it('update() calls wasmSdk.identityUpdate with options', async () => {
    const options = {
      identityId: 'i',
      addPublicKeys: [{ k: 1 }],
      disablePublicKeyIds: [10, 20],
      signer: { privateKeyWif: 'w' },
    };
    await client.identities.update(options);
    expect(wasmSdk.identityUpdate).to.be.calledOnceWithExactly(options);
  });
});
