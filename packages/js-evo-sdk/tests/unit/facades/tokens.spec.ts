import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('TokensFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;
  let identityKey: wasmSDKPackage.IdentityPublicKey;
  let signer: wasmSDKPackage.IdentitySigner;

  // Realistic identifiers
  const contractId = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
  const tokenId = 'BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus';
  const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const recipientId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';

  // Stub references for type-safe assertions
  let getTokenPriceByContractStub: SinonStub;
  let getTokenTotalSupplyStub: SinonStub;
  let getTokenTotalSupplyWithProofInfoStub: SinonStub;
  let getTokenStatusesStub: SinonStub;
  let getTokenStatusesWithProofInfoStub: SinonStub;
  let getIdentitiesTokenBalancesStub: SinonStub;
  let getIdentitiesTokenBalancesWithProofInfoStub: SinonStub;
  let getIdentityTokenBalancesStub: SinonStub;
  let getIdentityTokenBalancesWithProofInfoStub: SinonStub;
  let getIdentityTokenInfosStub: SinonStub;
  let getIdentitiesTokenInfosStub: SinonStub;
  let getIdentityTokenInfosWithProofInfoStub: SinonStub;
  let getIdentitiesTokenInfosWithProofInfoStub: SinonStub;
  let getTokenDirectPurchasePricesStub: SinonStub;
  let getTokenDirectPurchasePricesWithProofInfoStub: SinonStub;
  let getTokenContractInfoStub: SinonStub;
  let getTokenContractInfoWithProofInfoStub: SinonStub;
  let getTokenPerpetualDistributionLastClaimStub: SinonStub;
  let getTokenPerpetualDistributionLastClaimWithProofInfoStub: SinonStub;
  let tokenMintStub: SinonStub;
  let tokenBurnStub: SinonStub;
  let tokenTransferStub: SinonStub;
  let tokenFreezeStub: SinonStub;
  let tokenUnfreezeStub: SinonStub;
  let tokenDestroyFrozenStub: SinonStub;
  let tokenEmergencyActionStub: SinonStub;
  let tokenSetPriceStub: SinonStub;
  let tokenDirectPurchaseStub: SinonStub;
  let tokenClaimStub: SinonStub;
  let tokenConfigUpdateStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    getTokenPriceByContractStub = this.sinon.stub(wasmSdk, 'getTokenPriceByContract').resolves({
      price: BigInt(1000000),
      currencyId: tokenId,
    });
    getTokenTotalSupplyStub = this.sinon.stub(wasmSdk, 'getTokenTotalSupply').resolves({
      totalSupply: BigInt(1000000000),
      tokenId,
    });
    getTokenTotalSupplyWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTokenTotalSupplyWithProofInfo').resolves({
      data: { totalSupply: BigInt(1000000000), tokenId },
      proof: {},
      metadata: {},
    });
    getTokenStatusesStub = this.sinon.stub(wasmSdk, 'getTokenStatuses').resolves(new Map());
    getTokenStatusesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTokenStatusesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getIdentitiesTokenBalancesStub = this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalances').resolves(new Map());
    getIdentitiesTokenBalancesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getIdentityTokenBalancesStub = this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves(new Map());
    getIdentityTokenBalancesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getIdentityTokenInfosStub = this.sinon.stub(wasmSdk, 'getIdentityTokenInfos').resolves(new Map());
    getIdentitiesTokenInfosStub = this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfos').resolves(new Map());
    getIdentityTokenInfosWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityTokenInfosWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getIdentitiesTokenInfosWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfosWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getTokenDirectPurchasePricesStub = this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePrices').resolves(new Map());
    getTokenDirectPurchasePricesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePricesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getTokenContractInfoStub = this.sinon.stub(wasmSdk, 'getTokenContractInfo').resolves({
      contractId,
      tokenPosition: 0,
    });
    getTokenContractInfoWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTokenContractInfoWithProofInfo').resolves({
      data: { contractId, tokenPosition: 0 },
      proof: {},
      metadata: {},
    });
    getTokenPerpetualDistributionLastClaimStub = this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaim').resolves(undefined);
    getTokenPerpetualDistributionLastClaimWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaimWithProofInfo').resolves({
      data: undefined,
      proof: {},
      metadata: {},
    });

    // Stub transition methods - all return result objects
    tokenMintStub = this.sinon.stub(wasmSdk, 'tokenMint').resolves({
      tokenId,
      balance: BigInt(100000000),
    });
    tokenBurnStub = this.sinon.stub(wasmSdk, 'tokenBurn').resolves({
      tokenId,
      balance: BigInt(50000000),
    });
    tokenTransferStub = this.sinon.stub(wasmSdk, 'tokenTransfer').resolves({
      tokenId,
      senderBalance: BigInt(40000000),
      recipientBalance: BigInt(60000000),
    });
    tokenFreezeStub = this.sinon.stub(wasmSdk, 'tokenFreeze').resolves({ tokenId });
    tokenUnfreezeStub = this.sinon.stub(wasmSdk, 'tokenUnfreeze').resolves({ tokenId });
    tokenDestroyFrozenStub = this.sinon.stub(wasmSdk, 'tokenDestroyFrozen').resolves({ tokenId });
    tokenEmergencyActionStub = this.sinon.stub(wasmSdk, 'tokenEmergencyAction').resolves({ tokenId });
    tokenSetPriceStub = this.sinon.stub(wasmSdk, 'tokenSetPrice').resolves({ tokenId });
    tokenDirectPurchaseStub = this.sinon.stub(wasmSdk, 'tokenDirectPurchase').resolves({
      tokenId,
      balance: BigInt(10000000),
    });
    tokenClaimStub = this.sinon.stub(wasmSdk, 'tokenClaim').resolves({
      tokenId,
      claimedAmount: BigInt(5000000),
    });
    tokenConfigUpdateStub = this.sinon.stub(wasmSdk, 'tokenConfigUpdate').resolves({
      tokenId,
    });
  });

  describe('calculateId()', () => {
    it('should compute token ID from contract ID and position', async () => {
      const result = await client.tokens.calculateId(contractId, 0);
      expect(result).to.equal(tokenId);
    });
  });

  describe('priceByContract()', () => {
    it('should fetch token price by contract ID', async () => {
      const tokenPosition = 0;

      await client.tokens.priceByContract(contractId, tokenPosition);

      expect(getTokenPriceByContractStub)
        .to.be.calledOnceWithExactly(contractId, tokenPosition);
    });
  });

  describe('totalSupply()', () => {
    it('should fetch total supply of a token', async () => {
      await client.tokens.totalSupply(tokenId);

      expect(getTokenTotalSupplyStub).to.be.calledOnceWithExactly(tokenId);
    });
  });

  describe('totalSupplyWithProof()', () => {
    it('should fetch total supply with proof', async () => {
      await client.tokens.totalSupplyWithProof(tokenId);

      expect(getTokenTotalSupplyWithProofInfoStub).to.be.calledOnceWithExactly(tokenId);
    });
  });

  describe('statuses()', () => {
    it('should fetch statuses for multiple tokens', async () => {
      const tokenIds = [tokenId, 'AnotherTokenId123456789abcdefghijklmnop'];

      await client.tokens.statuses(tokenIds);

      expect(getTokenStatusesStub).to.be.calledOnceWithExactly(tokenIds);
    });
  });

  describe('statusesWithProof()', () => {
    it('should fetch token statuses with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.statusesWithProof(tokenIds);

      expect(getTokenStatusesWithProofInfoStub).to.be.calledOnceWithExactly(tokenIds);
    });
  });

  describe('balances()', () => {
    it('should fetch token balances for multiple identities', async () => {
      const identityIds = [identityId, recipientId];

      await client.tokens.balances(identityIds, tokenId);

      expect(getIdentitiesTokenBalancesStub).to.be.calledOnceWithExactly(identityIds, tokenId);
    });
  });

  describe('balancesWithProof()', () => {
    it('should fetch identity balances with proof', async () => {
      const identityIds = [identityId];

      await client.tokens.balancesWithProof(identityIds, tokenId);

      expect(getIdentitiesTokenBalancesWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityIds, tokenId);
    });
  });

  describe('identityBalances()', () => {
    it('should fetch balances for multiple tokens of one identity', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityBalances(identityId, tokenIds);

      expect(getIdentityTokenBalancesStub).to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('identityBalancesWithProof()', () => {
    it('should fetch identity token balances with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityBalancesWithProof(identityId, tokenIds);

      expect(getIdentityTokenBalancesWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('identityTokenInfos()', () => {
    it('should fetch token info for an identity', async () => {
      const tokenIds = [tokenId, 'AnotherTokenId123456789abcdefghijklmnop'];

      await client.tokens.identityTokenInfos(identityId, tokenIds);

      expect(getIdentityTokenInfosStub).to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('identitiesTokenInfos()', () => {
    it('should fetch token info for multiple identities', async () => {
      const identityIds = [identityId];

      await client.tokens.identitiesTokenInfos(identityIds, tokenId);

      expect(getIdentitiesTokenInfosStub).to.be.calledOnceWithExactly(identityIds, tokenId);
    });
  });

  describe('identityTokenInfosWithProof()', () => {
    it('should fetch token info with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityTokenInfosWithProof(identityId, tokenIds);

      expect(getIdentityTokenInfosWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, tokenIds);
    });
  });

  describe('identitiesTokenInfosWithProof()', () => {
    it('should fetch multiple identities info with proof', async () => {
      const identityIds = [identityId];

      await client.tokens.identitiesTokenInfosWithProof(identityIds, tokenId);

      expect(getIdentitiesTokenInfosWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityIds, tokenId);
    });
  });

  describe('directPurchasePrices()', () => {
    it('should fetch purchase prices for tokens', async () => {
      const tokenIds = [tokenId];

      await client.tokens.directPurchasePrices(tokenIds);

      expect(getTokenDirectPurchasePricesStub).to.be.calledOnceWithExactly(tokenIds);
    });
  });

  describe('directPurchasePricesWithProof()', () => {
    it('should fetch purchase prices with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.directPurchasePricesWithProof(tokenIds);

      expect(getTokenDirectPurchasePricesWithProofInfoStub)
        .to.be.calledOnceWithExactly(tokenIds);
    });
  });

  describe('contractInfo()', () => {
    it('should fetch token contract information', async () => {
      await client.tokens.contractInfo(contractId);

      expect(getTokenContractInfoStub).to.be.calledOnceWithExactly(contractId);
    });
  });

  describe('contractInfoWithProof()', () => {
    it('should fetch contract info with proof', async () => {
      await client.tokens.contractInfoWithProof(contractId);

      expect(getTokenContractInfoWithProofInfoStub).to.be.calledOnceWithExactly(contractId);
    });
  });

  describe('perpetualDistributionLastClaim()', () => {
    it('should fetch last claim time', async () => {
      await client.tokens.perpetualDistributionLastClaim(identityId, tokenId);

      expect(getTokenPerpetualDistributionLastClaimStub)
        .to.be.calledOnceWithExactly(identityId, tokenId);
    });
  });

  describe('perpetualDistributionLastClaimWithProof()', () => {
    it('should fetch last claim with proof', async () => {
      await client.tokens.perpetualDistributionLastClaimWithProof(identityId, tokenId);

      expect(getTokenPerpetualDistributionLastClaimWithProofInfoStub)
        .to.be.calledOnceWithExactly(identityId, tokenId);
    });
  });

  describe('mint()', () => {
    it('should mint new tokens to an identity', async () => {
      const options = {
        tokenId,
        amount: BigInt(50000000), // 50M tokens
        recipientId,
        identityKey,
        signer,
        publicNote: 'Initial token distribution',
      };

      const result = await client.tokens.mint(options);

      expect(tokenMintStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(100000000));
    });
  });

  describe('burn()', () => {
    it('should burn tokens from an identity', async () => {
      const options = {
        tokenId,
        amount: BigInt(10000000), // 10M tokens
        identityKey,
        signer,
        publicNote: 'Token buyback and burn',
      };

      const result = await client.tokens.burn(options);

      expect(tokenBurnStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(50000000));
    });
  });

  describe('transfer()', () => {
    it('should transfer tokens between identities', async () => {
      const options = {
        tokenId,
        amount: BigInt(25000000), // 25M tokens
        recipientId,
        identityKey,
        signer,
        publicNote: 'Payment for services',
      };

      const result = await client.tokens.transfer(options);

      expect(tokenTransferStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.senderBalance).to.equal(BigInt(40000000));
      expect(result.recipientBalance).to.equal(BigInt(60000000));
    });
  });

  describe('freeze()', () => {
    it('should freeze tokens for an identity', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Account frozen for compliance review',
      };

      const result = await client.tokens.freeze(options);

      expect(tokenFreezeStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });

  describe('unfreeze()', () => {
    it('should unfreeze previously frozen tokens', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Compliance review completed',
      };

      const result = await client.tokens.unfreeze(options);

      expect(tokenUnfreezeStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });

  describe('destroyFrozen()', () => {
    it('should destroy frozen tokens', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Fraudulent tokens destroyed',
      };

      const result = await client.tokens.destroyFrozen(options);

      expect(tokenDestroyFrozenStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });

  describe('emergencyAction()', () => {
    it('should execute emergency token action', async () => {
      const options = {
        tokenId,
        action: 'pause',
        identityKey,
        signer,
        publicNote: 'Emergency pause due to security concern',
      };

      const result = await client.tokens.emergencyAction(options);

      expect(tokenEmergencyActionStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });

  describe('setPrice()', () => {
    it('should set direct purchase price for tokens', async () => {
      const options = {
        tokenId,
        price: {
          type: 'fixed',
          value: BigInt(1000000), // 1M credits per token
        },
        identityKey,
        signer,
      };

      const result = await client.tokens.setPrice(options);

      expect(tokenSetPriceStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });

  describe('directPurchase()', () => {
    it('should purchase tokens directly', async () => {
      const options = {
        tokenId,
        amount: BigInt(5000000), // 5M tokens
        totalAgreedPrice: BigInt(5000000000), // 5B credits
        identityKey,
        signer,
      };

      const result = await client.tokens.directPurchase(options);

      expect(tokenDirectPurchaseStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(10000000));
    });
  });

  describe('claim()', () => {
    it('should claim token distribution rewards', async () => {
      const options = {
        tokenId,
        identityKey,
        signer,
        publicNote: 'Claiming weekly distribution',
      };

      const result = await client.tokens.claim(options);

      expect(tokenClaimStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.claimedAmount).to.equal(BigInt(5000000));
    });
  });

  describe('configUpdate()', () => {
    it('should update token configuration', async () => {
      const options = {
        tokenId,
        configurationChangeItem: { type: 'MaxSupply', value: BigInt(1000000000) },
        identityKey,
        signer,
        publicNote: 'Updating max supply',
      };

      const result = await client.tokens.configUpdate(options);

      expect(tokenConfigUpdateStub).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });
  });
});
