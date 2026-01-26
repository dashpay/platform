import { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('IdentitiesFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;
  let identity: wasmSDKPackage.Identity;
  let signer: wasmSDKPackage.IdentitySigner;
  let assetLockProof: wasmSDKPackage.AssetLockProof;
  let assetLockPrivateKey: wasmSDKPackage.PrivateKey;
  let publicKeyInCreation: wasmSDKPackage.IdentityPublicKeyInCreation;

  // Stub references for type-safe assertions
  let getIdentityStub: SinonStub;
  let getIdentityWithProofInfoStub: SinonStub;
  let getIdentityUnprovedStub: SinonStub;
  let getIdentityKeysStub: SinonStub;
  let getIdentityKeysWithProofInfoStub: SinonStub;
  let getIdentityNonceStub: SinonStub;
  let getIdentityNonceWithProofInfoStub: SinonStub;
  let getIdentityContractNonceStub: SinonStub;
  let getIdentityContractNonceWithProofInfoStub: SinonStub;
  let getIdentityBalanceStub: SinonStub;
  let getIdentityBalanceWithProofInfoStub: SinonStub;
  let getIdentitiesBalancesStub: SinonStub;
  let getIdentitiesBalancesWithProofInfoStub: SinonStub;
  let getIdentityBalanceAndRevisionStub: SinonStub;
  let getIdentityBalanceAndRevisionWithProofInfoStub: SinonStub;
  let getIdentityByPublicKeyHashStub: SinonStub;
  let getIdentityByPublicKeyHashWithProofInfoStub: SinonStub;
  let getIdentityByNonUniquePublicKeyHashStub: SinonStub;
  let getIdentityByNonUniquePublicKeyHashWithProofInfoStub: SinonStub;
  let getIdentitiesContractKeysStub: SinonStub;
  let getIdentitiesContractKeysWithProofInfoStub: SinonStub;
  let getIdentityTokenBalancesStub: SinonStub;
  let getIdentityTokenBalancesWithProofInfoStub: SinonStub;
  let identityCreateStub: SinonStub;
  let identityTopUpStub: SinonStub;
  let identityCreditTransferStub: SinonStub;
  let identityCreditWithdrawalStub: SinonStub;
  let identityUpdateStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    identity = Object.create(wasmSDKPackage.Identity.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);
    assetLockProof = Object.create(wasmSDKPackage.AssetLockProof.prototype);
    assetLockPrivateKey = Object.create(wasmSDKPackage.PrivateKey.prototype);
    publicKeyInCreation = Object.create(wasmSDKPackage.IdentityPublicKeyInCreation.prototype);

    // Stub query methods
    getIdentityStub = this.sinon.stub(wasmSdk, 'getIdentity').resolves(identity);
    getIdentityWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityWithProofInfo').resolves({
      data: identity,
      proof: {},
      metadata: {},
    });
    getIdentityUnprovedStub = this.sinon.stub(wasmSdk, 'getIdentityUnproved').resolves(identity);
    getIdentityKeysStub = this.sinon.stub(wasmSdk, 'getIdentityKeys').resolves([]);
    getIdentityKeysWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityKeysWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    getIdentityNonceStub = this.sinon.stub(wasmSdk, 'getIdentityNonce').resolves(BigInt(1));
    getIdentityNonceWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityNonceWithProofInfo').resolves({
      data: BigInt(1),
      proof: {},
      metadata: {},
    });
    getIdentityContractNonceStub = this.sinon.stub(wasmSdk, 'getIdentityContractNonce').resolves(BigInt(0));
    getIdentityContractNonceWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityContractNonceWithProofInfo').resolves({
      data: BigInt(0),
      proof: {},
      metadata: {},
    });
    getIdentityBalanceStub = this.sinon.stub(wasmSdk, 'getIdentityBalance').resolves(BigInt(100000000));
    getIdentityBalanceWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityBalanceWithProofInfo').resolves({
      data: BigInt(100000000),
      proof: {},
      metadata: {},
    });
    getIdentitiesBalancesStub = this.sinon.stub(wasmSdk, 'getIdentitiesBalances').resolves(new Map());
    getIdentitiesBalancesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentitiesBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getIdentityBalanceAndRevisionStub = this.sinon.stub(wasmSdk, 'getIdentityBalanceAndRevision').resolves({
      balance: BigInt(100000000),
      revision: BigInt(1),
    });
    getIdentityBalanceAndRevisionWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentityBalanceAndRevisionWithProofInfo').resolves({
        data: { balance: BigInt(100000000), revision: BigInt(1) },
        proof: {},
        metadata: {},
      });
    getIdentityByPublicKeyHashStub = this.sinon.stub(wasmSdk, 'getIdentityByPublicKeyHash').resolves(identity);
    getIdentityByPublicKeyHashWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentityByPublicKeyHashWithProofInfo').resolves({
        data: identity,
        proof: {},
        metadata: {},
      });
    getIdentityByNonUniquePublicKeyHashStub = this.sinon.stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHash').resolves([]);
    getIdentityByNonUniquePublicKeyHashWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHashWithProofInfo').resolves({
        data: [],
        proof: {},
        metadata: {},
      });
    getIdentitiesContractKeysStub = this.sinon.stub(wasmSdk, 'getIdentitiesContractKeys').resolves([]);
    getIdentitiesContractKeysWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentitiesContractKeysWithProofInfo').resolves({
        data: [],
        proof: {},
        metadata: {},
      });
    getIdentityTokenBalancesStub = this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves(new Map());
    getIdentityTokenBalancesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    identityCreateStub = this.sinon.stub(wasmSdk, 'identityCreate').resolves();
    identityTopUpStub = this.sinon.stub(wasmSdk, 'identityTopUp').resolves(BigInt(200000000));
    identityCreditTransferStub = this.sinon.stub(wasmSdk, 'identityCreditTransfer').resolves({
      senderBalance: BigInt(50000000),
      recipientBalance: BigInt(50000000),
    });
    identityCreditWithdrawalStub = this.sinon.stub(wasmSdk, 'identityCreditWithdrawal').resolves(BigInt(80000000));
    identityUpdateStub = this.sinon.stub(wasmSdk, 'identityUpdate').resolves();
  });

  describe('Query Methods', () => {
    it('fetch() returns an identity by ID', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      const result = await client.identities.fetch(identityId);

      expect(getIdentityStub).to.be.calledOnceWithExactly(identityId);
      expect(result).to.be.instanceOf(wasmSDKPackage.Identity);
    });

    it('fetchWithProof() returns identity with proof metadata', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchWithProof(identityId);

      expect(getIdentityWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
    });

    it('fetchUnproved() returns identity without proof verification', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchUnproved(identityId);

      expect(getIdentityUnprovedStub).to.be.calledOnceWithExactly(identityId);
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

      expect(getIdentityKeysStub).to.be.calledOnceWithExactly(query);
    });

    it('getKeysWithProof() fetches identity keys with proof', async () => {
      const query = {
        identityId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        request: { type: 'all' },
      };

      await client.identities.getKeysWithProof(query);

      expect(getIdentityKeysWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });

    it('nonce() and nonceWithProof() fetch identity nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.nonce(identityId);
      await client.identities.nonceWithProof(identityId);

      expect(getIdentityNonceStub).to.be.calledOnceWithExactly(identityId);
      expect(getIdentityNonceWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
    });

    it('contractNonce() fetches contract-specific nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.identities.contractNonce(identityId, contractId);
      await client.identities.contractNonceWithProof(identityId, contractId);

      expect(getIdentityContractNonceStub)
        .to.be.calledOnceWithExactly(identityId, contractId);
      expect(getIdentityContractNonceWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, contractId);
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

      expect(getIdentityBalanceStub).to.be.calledOnceWithExactly(identityId);
      expect(getIdentityBalanceWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
      expect(getIdentitiesBalancesStub).to.be.calledOnceWithExactly(identityIds);
      expect(getIdentitiesBalancesWithProofInfoStub).to.be.calledOnceWithExactly(identityIds);
    });

    it('balanceAndRevision() fetches balance and revision together', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balanceAndRevision(identityId);
      await client.identities.balanceAndRevisionWithProof(identityId);

      expect(getIdentityBalanceAndRevisionStub)
        .to.be.calledOnceWithExactly(identityId);
      expect(getIdentityBalanceAndRevisionWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId);
    });

    it('byPublicKeyHash() looks up identity by public key hash', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

      await client.identities.byPublicKeyHash(publicKeyHash);
      await client.identities.byPublicKeyHashWithProof(publicKeyHash);

      expect(getIdentityByPublicKeyHashStub)
        .to.be.calledOnceWithExactly(publicKeyHash);
      expect(getIdentityByPublicKeyHashWithProofInfoStub)
        .to.be.calledOnceWithExactly(publicKeyHash);
    });

    it('byNonUniquePublicKeyHash() supports pagination cursor', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';
      const startAfter = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.byNonUniquePublicKeyHash(publicKeyHash, startAfter);
      await client.identities.byNonUniquePublicKeyHashWithProof(publicKeyHash);

      expect(getIdentityByNonUniquePublicKeyHashStub)
        .to.be.calledOnceWithExactly(publicKeyHash, startAfter);
      expect(getIdentityByNonUniquePublicKeyHashWithProofInfoStub)
        .to.be.calledOnceWithExactly(publicKeyHash, undefined);
    });

    it('contractKeys() fetches contract-bound keys for identities', async () => {
      const query = {
        identityIds: ['5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS'],
        contractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        purposes: [0, 1], // AUTHENTICATION, ENCRYPTION
      };

      await client.identities.contractKeys(query);

      expect(getIdentitiesContractKeysStub).to.be.calledOnceWithExactly(query);
    });

    it('tokenBalances() fetches identity token balances', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const tokenIds = ['BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus'];

      await client.identities.tokenBalances(identityId, tokenIds);
      await client.identities.tokenBalancesWithProof(identityId, tokenIds);

      expect(getIdentityTokenBalancesStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
      expect(getIdentityTokenBalancesWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
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

      expect(identityCreateStub).to.be.calledOnceWithExactly(options);
    });

    it('topUp() tops up identity balance with asset lock', async () => {
      const options = {
        identity,
        assetLockProof,
        assetLockPrivateKey,
      };

      const newBalance = await client.identities.topUp(options);

      expect(identityTopUpStub).to.be.calledOnceWithExactly(options);
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

      expect(identityCreditTransferStub).to.be.calledOnceWithExactly(options);
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

      expect(identityCreditWithdrawalStub).to.be.calledOnceWithExactly(options);
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

      expect(identityUpdateStub).to.be.calledOnceWithExactly(options);
    });
  });
});
