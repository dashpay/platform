import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('IdentitiesFacade', () => {
  let wasmSdk;
  let client;
  let identity;
  let identityKey;
  let signer;
  let assetLockProof;
  let assetLockPrivateKey;
  let publicKeyInCreation;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    identity = Object.create(wasmSDKPackage.Identity.prototype);
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);
    assetLockProof = Object.create(wasmSDKPackage.AssetLockProof.prototype);
    assetLockPrivateKey = Object.create(wasmSDKPackage.PrivateKey.prototype);
    publicKeyInCreation = Object.create(wasmSDKPackage.IdentityPublicKeyInCreation.prototype);

    // Stub query methods
    this.sinon.stub(wasmSdk, 'getIdentity').resolves(identity);
    this.sinon.stub(wasmSdk, 'getIdentityWithProofInfo').resolves({
      data: identity,
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityUnproved').resolves(identity);
    this.sinon.stub(wasmSdk, 'getIdentityKeys').resolves([]);
    this.sinon.stub(wasmSdk, 'getIdentityKeysWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityNonce').resolves(BigInt(1));
    this.sinon.stub(wasmSdk, 'getIdentityNonceWithProofInfo').resolves({
      data: BigInt(1),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityContractNonce').resolves(BigInt(0));
    this.sinon.stub(wasmSdk, 'getIdentityContractNonceWithProofInfo').resolves({
      data: BigInt(0),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityBalance').resolves(BigInt(100000000));
    this.sinon.stub(wasmSdk, 'getIdentityBalanceWithProofInfo').resolves({
      data: BigInt(100000000),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentitiesBalances').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentitiesBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityBalanceAndRevision').resolves({
      balance: BigInt(100000000),
      revision: BigInt(1),
    });
    this.sinon.stub(wasmSdk, 'getIdentityBalanceAndRevisionWithProofInfo').resolves({
      data: { balance: BigInt(100000000), revision: BigInt(1) },
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityByPublicKeyHash').resolves(identity);
    this.sinon.stub(wasmSdk, 'getIdentityByPublicKeyHashWithProofInfo').resolves({
      data: identity,
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHash').resolves([]);
    this.sinon.stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHashWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentitiesContractKeys').resolves([]);
    this.sinon.stub(wasmSdk, 'getIdentitiesContractKeysWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    this.sinon.stub(wasmSdk, 'identityCreate').resolves();
    this.sinon.stub(wasmSdk, 'identityTopUp').resolves(BigInt(200000000));
    this.sinon.stub(wasmSdk, 'identityCreditTransfer').resolves({
      senderBalance: BigInt(50000000),
      recipientBalance: BigInt(50000000),
    });
    this.sinon.stub(wasmSdk, 'identityCreditWithdrawal').resolves(BigInt(80000000));
    this.sinon.stub(wasmSdk, 'identityUpdate').resolves();
  });

  describe('Query Methods', () => {
    it('fetch() returns an identity by ID', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      const result = await client.identities.fetch(identityId);

      expect(wasmSdk.getIdentity).to.be.calledOnceWithExactly(identityId);
      expect(result).to.be.instanceOf(wasmSDKPackage.Identity);
    });

    it('fetchWithProof() returns identity with proof metadata', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchWithProof(identityId);

      expect(wasmSdk.getIdentityWithProofInfo).to.be.calledOnceWithExactly(identityId);
    });

    it('fetchUnproved() returns identity without proof verification', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchUnproved(identityId);

      expect(wasmSdk.getIdentityUnproved).to.be.calledOnceWithExactly(identityId);
    });

    it('getKeys() fetches identity public keys', async () => {
      const query = {
        identityId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        request: {
          type: 'specific',
          specificKeyIds: [0, 1],
        },
        limit: 10,
        offset: 0,
      };

      await client.identities.getKeys(query);

      expect(wasmSdk.getIdentityKeys).to.be.calledOnceWithExactly(query);
    });

    it('getKeysWithProof() fetches identity keys with proof', async () => {
      const query = {
        identityId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        request: { type: 'all' },
      };

      await client.identities.getKeysWithProof(query);

      expect(wasmSdk.getIdentityKeysWithProofInfo).to.be.calledOnceWithExactly(query);
    });

    it('nonce() and nonceWithProof() fetch identity nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.nonce(identityId);
      await client.identities.nonceWithProof(identityId);

      expect(wasmSdk.getIdentityNonce).to.be.calledOnceWithExactly(identityId);
      expect(wasmSdk.getIdentityNonceWithProofInfo).to.be.calledOnceWithExactly(identityId);
    });

    it('contractNonce() fetches contract-specific nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.identities.contractNonce(identityId, contractId);
      await client.identities.contractNonceWithProof(identityId, contractId);

      expect(wasmSdk.getIdentityContractNonce).to.be.calledOnceWithExactly(identityId, contractId);
      expect(wasmSdk.getIdentityContractNonceWithProofInfo).to.be.calledOnceWithExactly(identityId, contractId);
    });

    it('balance() and balances() fetch identity credits', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const identityIds = [
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack',
      ];

      await client.identities.balance(identityId);
      await client.identities.balanceWithProof(identityId);
      await client.identities.balances(identityIds);
      await client.identities.balancesWithProof(identityIds);

      expect(wasmSdk.getIdentityBalance).to.be.calledOnceWithExactly(identityId);
      expect(wasmSdk.getIdentityBalanceWithProofInfo).to.be.calledOnceWithExactly(identityId);
      expect(wasmSdk.getIdentitiesBalances).to.be.calledOnceWithExactly(identityIds);
      expect(wasmSdk.getIdentitiesBalancesWithProofInfo).to.be.calledOnceWithExactly(identityIds);
    });

    it('balanceAndRevision() fetches balance and revision together', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balanceAndRevision(identityId);
      await client.identities.balanceAndRevisionWithProof(identityId);

      expect(wasmSdk.getIdentityBalanceAndRevision).to.be.calledOnceWithExactly(identityId);
      expect(wasmSdk.getIdentityBalanceAndRevisionWithProofInfo).to.be.calledOnceWithExactly(identityId);
    });

    it('byPublicKeyHash() looks up identity by public key hash', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

      await client.identities.byPublicKeyHash(publicKeyHash);
      await client.identities.byPublicKeyHashWithProof(publicKeyHash);

      expect(wasmSdk.getIdentityByPublicKeyHash).to.be.calledOnceWithExactly(publicKeyHash);
      expect(wasmSdk.getIdentityByPublicKeyHashWithProofInfo).to.be.calledOnceWithExactly(publicKeyHash);
    });

    it('byNonUniquePublicKeyHash() supports pagination cursor', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';
      const startAfter = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.byNonUniquePublicKeyHash(publicKeyHash, startAfter);
      await client.identities.byNonUniquePublicKeyHashWithProof(publicKeyHash);

      expect(wasmSdk.getIdentityByNonUniquePublicKeyHash).to.be.calledOnceWithExactly(publicKeyHash, startAfter);
      expect(wasmSdk.getIdentityByNonUniquePublicKeyHashWithProofInfo).to.be.calledOnceWithExactly(publicKeyHash, undefined);
    });

    it('contractKeys() fetches contract-bound keys for identities', async () => {
      const query = {
        identityIds: ['5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS'],
        contractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        purposes: [0, 1], // AUTHENTICATION, ENCRYPTION
      };

      await client.identities.contractKeys(query);

      expect(wasmSdk.getIdentitiesContractKeys).to.be.calledOnceWithExactly(query);
    });

    it('tokenBalances() fetches identity token balances', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const tokenIds = ['BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus'];

      await client.identities.tokenBalances(identityId, tokenIds);
      await client.identities.tokenBalancesWithProof(identityId, tokenIds);

      expect(wasmSdk.getIdentityTokenBalances).to.be.calledOnceWithExactly(identityId, tokenIds);
      expect(wasmSdk.getIdentityTokenBalancesWithProofInfo).to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('Transition Methods', () => {
    it('create() creates a new identity with asset lock', async () => {
      const options = {
        identity,
        assetLockProof,
        assetLockPrivateKey,
        signer,
      };

      await client.identities.create(options);

      expect(wasmSdk.identityCreate).to.be.calledOnceWithExactly(options);
    });

    it('topUp() tops up identity balance with asset lock', async () => {
      const options = {
        identity,
        assetLockProof,
        assetLockPrivateKey,
      };

      const newBalance = await client.identities.topUp(options);

      expect(wasmSdk.identityTopUp).to.be.calledOnceWithExactly(options);
      expect(newBalance).to.equal(BigInt(200000000));
    });

    it('creditTransfer() transfers credits between identities', async () => {
      const recipientId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
      const options = {
        identity,
        recipientId,
        amount: BigInt(50000000), // 50M credits
        signer,
      };

      const result = await client.identities.creditTransfer(options);

      expect(wasmSdk.identityCreditTransfer).to.be.calledOnceWithExactly(options);
      expect(result.senderBalance).to.equal(BigInt(50000000));
      expect(result.recipientBalance).to.equal(BigInt(50000000));
    });

    it('creditWithdrawal() withdraws credits to Dash address', async () => {
      const options = {
        identity,
        amount: BigInt(20000000), // 20M credits
        toAddress: 'yNPbcFfabt8MMPjYjWBGjpAJWYhUMoqoUo',
        coreFeePerByte: 1,
        signer,
      };

      const remainingBalance = await client.identities.creditWithdrawal(options);

      expect(wasmSdk.identityCreditWithdrawal).to.be.calledOnceWithExactly(options);
      expect(remainingBalance).to.equal(BigInt(80000000));
    });

    it('update() adds and disables public keys', async () => {
      const options = {
        identity,
        addPublicKeys: [publicKeyInCreation],
        disablePublicKeys: [2, 3],
        signer,
      };

      await client.identities.update(options);

      expect(wasmSdk.identityUpdate).to.be.calledOnceWithExactly(options);
    });
  });
});
