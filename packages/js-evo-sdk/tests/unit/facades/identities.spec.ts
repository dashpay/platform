import type { SinonStub } from 'sinon';
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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
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
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
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
    getIdentityContractNonceWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentityContractNonceWithProofInfo').resolves({
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
    getIdentityByNonUniquePublicKeyHashStub = this.sinon
      .stub(wasmSdk, 'getIdentityByNonUniquePublicKeyHash').resolves([]);
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
    getIdentityTokenBalancesWithProofInfoStub = this.sinon
      .stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves({
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

  describe('fetch()', () => {
    it('should return an identity by ID', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      const result = await client.identities.fetch(identityId);

      expect(getIdentityStub).to.be.calledOnceWithExactly(identityId);
      expect(result).to.be.instanceOf(wasmSDKPackage.Identity);
    });
  });

  describe('fetchWithProof()', () => {
    it('should return identity with proof metadata', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchWithProof(identityId);

      expect(getIdentityWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('fetchUnproved()', () => {
    it('should return identity without proof verification', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.fetchUnproved(identityId);

      expect(getIdentityUnprovedStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('getKeys()', () => {
    it('should fetch identity public keys', async () => {
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
  });

  describe('getKeysWithProof()', () => {
    it('should fetch identity keys with proof', async () => {
      const query = {
        identityId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        request: { type: 'all' },
      };

      await client.identities.getKeysWithProof(query);

      expect(getIdentityKeysWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('nonce()', () => {
    it('should fetch identity nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.nonce(identityId);

      expect(getIdentityNonceStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('nonceWithProof()', () => {
    it('should fetch identity nonce with proof', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.nonceWithProof(identityId);

      expect(getIdentityNonceWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('contractNonce()', () => {
    it('should fetch contract-specific nonce', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.identities.contractNonce(identityId, contractId);

      expect(getIdentityContractNonceStub)
        .to.be.calledOnceWithExactly(identityId, contractId);
    });
  });

  describe('contractNonceWithProof()', () => {
    it('should fetch contract-specific nonce with proof', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.identities.contractNonceWithProof(identityId, contractId);

      expect(getIdentityContractNonceWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, contractId);
    });
  });

  describe('balance()', () => {
    it('should fetch identity credits balance', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balance(identityId);

      expect(getIdentityBalanceStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('balanceWithProof()', () => {
    it('should fetch identity credits balance with proof', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balanceWithProof(identityId);

      expect(getIdentityBalanceWithProofInfoStub).to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('balances()', () => {
    it('should fetch balances for multiple identities', async () => {
      const identityIds = [
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack',
      ];

      await client.identities.balances(identityIds);

      expect(getIdentitiesBalancesStub).to.be.calledOnceWithExactly(identityIds);
    });
  });

  describe('balancesWithProof()', () => {
    it('should fetch balances for multiple identities with proof', async () => {
      const identityIds = [
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
        '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack',
      ];

      await client.identities.balancesWithProof(identityIds);

      expect(getIdentitiesBalancesWithProofInfoStub).to.be.calledOnceWithExactly(identityIds);
    });
  });

  describe('balanceAndRevision()', () => {
    it('should fetch balance and revision together', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balanceAndRevision(identityId);

      expect(getIdentityBalanceAndRevisionStub)
        .to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('balanceAndRevisionWithProof()', () => {
    it('should fetch balance and revision with proof', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.balanceAndRevisionWithProof(identityId);

      expect(getIdentityBalanceAndRevisionWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId);
    });
  });

  describe('byPublicKeyHash()', () => {
    it('should look up identity by public key hash', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

      await client.identities.byPublicKeyHash(publicKeyHash);

      expect(getIdentityByPublicKeyHashStub)
        .to.be.calledOnceWithExactly(publicKeyHash);
    });
  });

  describe('byPublicKeyHashWithProof()', () => {
    it('should look up identity by public key hash with proof', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

      await client.identities.byPublicKeyHashWithProof(publicKeyHash);

      expect(getIdentityByPublicKeyHashWithProofInfoStub)
        .to.be.calledOnceWithExactly(publicKeyHash);
    });
  });

  describe('byNonUniquePublicKeyHash()', () => {
    it('should support pagination cursor for non-unique public key hash lookup', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';
      const startAfter = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';

      await client.identities.byNonUniquePublicKeyHash(publicKeyHash, startAfter);

      expect(getIdentityByNonUniquePublicKeyHashStub)
        .to.be.calledOnceWithExactly(publicKeyHash, startAfter);
    });
  });

  describe('byNonUniquePublicKeyHashWithProof()', () => {
    it('should look up by non-unique public key hash with proof', async () => {
      const publicKeyHash = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

      await client.identities.byNonUniquePublicKeyHashWithProof(publicKeyHash);

      expect(getIdentityByNonUniquePublicKeyHashWithProofInfoStub)
        .to.be.calledOnceWithExactly(publicKeyHash, undefined);
    });
  });

  describe('contractKeys()', () => {
    it('should fetch contract-bound keys for identities', async () => {
      const query = {
        identityIds: ['5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS'],
        contractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        purposes: [0, 1], // AUTHENTICATION, ENCRYPTION
      };

      await client.identities.contractKeys(query);

      expect(getIdentitiesContractKeysStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('tokenBalances()', () => {
    it('should fetch identity token balances', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const tokenIds = ['BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus'];

      await client.identities.tokenBalances(identityId, tokenIds);

      expect(getIdentityTokenBalancesStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('tokenBalancesWithProof()', () => {
    it('should fetch identity token balances with proof', async () => {
      const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
      const tokenIds = ['BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus'];

      await client.identities.tokenBalancesWithProof(identityId, tokenIds);

      expect(getIdentityTokenBalancesWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('create()', () => {
    it('should create a new identity with asset lock', async () => {
      const options = {
        identity,
        assetLockProof,
        assetLockPrivateKey,
        signer,
      };

      await client.identities.create(options);

      expect(identityCreateStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('topUp()', () => {
    it('should top up identity balance with asset lock', async () => {
      const options = {
        identity,
        assetLockProof,
        assetLockPrivateKey,
      };

      const newBalance = await client.identities.topUp(options);

      expect(identityTopUpStub).to.be.calledOnceWithExactly(options);
      expect(newBalance).to.equal(BigInt(200000000));
    });
  });

  describe('creditTransfer()', () => {
    it('should transfer credits between identities', async () => {
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
  });

  describe('creditWithdrawal()', () => {
    it('should withdraw credits to Dash address', async () => {
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
  });

  describe('update()', () => {
    it('should add and disable public keys', async () => {
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
