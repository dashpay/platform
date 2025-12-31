import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('TokensFacade', () => {
  let wasmSdk;
  let client;
  let identityKey;
  let signer;

  // Realistic identifiers
  const contractId = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
  const tokenId = 'BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus';
  const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const recipientId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    this.sinon.stub(wasmSdk, 'getTokenPriceByContract').resolves({
      price: BigInt(1000000),
      currencyId: tokenId,
    });
    this.sinon.stub(wasmSdk, 'getTokenTotalSupply').resolves({
      totalSupply: BigInt(1000000000),
      tokenId,
    });
    this.sinon.stub(wasmSdk, 'getTokenTotalSupplyWithProofInfo').resolves({
      data: { totalSupply: BigInt(1000000000), tokenId },
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getTokenStatuses').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getTokenStatusesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalances').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentityTokenInfos').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfos').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getIdentityTokenInfosWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfosWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePrices').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePricesWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getTokenContractInfo').resolves({
      contractId,
      tokenPosition: 0,
    });
    this.sinon.stub(wasmSdk, 'getTokenContractInfoWithProofInfo').resolves({
      data: { contractId, tokenPosition: 0 },
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaim').resolves(undefined);
    this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaimWithProofInfo').resolves({
      data: undefined,
      proof: {},
      metadata: {},
    });

    // Stub transition methods - all return result objects
    this.sinon.stub(wasmSdk, 'tokenMint').resolves({
      tokenId,
      balance: BigInt(100000000),
    });
    this.sinon.stub(wasmSdk, 'tokenBurn').resolves({
      tokenId,
      balance: BigInt(50000000),
    });
    this.sinon.stub(wasmSdk, 'tokenTransfer').resolves({
      tokenId,
      senderBalance: BigInt(40000000),
      recipientBalance: BigInt(60000000),
    });
    this.sinon.stub(wasmSdk, 'tokenFreeze').resolves({ tokenId });
    this.sinon.stub(wasmSdk, 'tokenUnfreeze').resolves({ tokenId });
    this.sinon.stub(wasmSdk, 'tokenDestroyFrozen').resolves({ tokenId });
    this.sinon.stub(wasmSdk, 'tokenEmergencyAction').resolves({ tokenId });
    this.sinon.stub(wasmSdk, 'tokenSetPrice').resolves({ tokenId });
    this.sinon.stub(wasmSdk, 'tokenDirectPurchase').resolves({
      tokenId,
      balance: BigInt(10000000),
    });
    this.sinon.stub(wasmSdk, 'tokenClaim').resolves({
      tokenId,
      claimedAmount: BigInt(5000000),
    });
  });

  describe('Static Methods', () => {
    it('calculateId() computes token ID from contract ID and position', async () => {
      const result = await client.tokens.calculateId(contractId, 0);
      expect(result).to.equal(tokenId);
    });
  });

  describe('Query Methods', () => {
    it('priceByContract() fetches token price by contract ID', async () => {
      const tokenPosition = 0;

      await client.tokens.priceByContract(contractId, tokenPosition);

      expect(wasmSdk.getTokenPriceByContract).to.be.calledOnceWithExactly(contractId, tokenPosition);
    });

    it('totalSupply() fetches total supply of a token', async () => {
      await client.tokens.totalSupply(tokenId);

      expect(wasmSdk.getTokenTotalSupply).to.be.calledOnceWithExactly(tokenId);
    });

    it('totalSupplyWithProof() fetches total supply with proof', async () => {
      await client.tokens.totalSupplyWithProof(tokenId);

      expect(wasmSdk.getTokenTotalSupplyWithProofInfo).to.be.calledOnceWithExactly(tokenId);
    });

    it('statuses() fetches statuses for multiple tokens', async () => {
      const tokenIds = [tokenId, 'AnotherTokenId123456789abcdefghijklmnop'];

      await client.tokens.statuses(tokenIds);

      expect(wasmSdk.getTokenStatuses).to.be.calledOnceWithExactly(tokenIds);
    });

    it('statusesWithProof() fetches token statuses with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.statusesWithProof(tokenIds);

      expect(wasmSdk.getTokenStatusesWithProofInfo).to.be.calledOnceWithExactly(tokenIds);
    });

    it('balances() fetches token balances for multiple identities', async () => {
      const identityIds = [identityId, recipientId];

      await client.tokens.balances(identityIds, tokenId);

      expect(wasmSdk.getIdentitiesTokenBalances).to.be.calledOnceWithExactly(identityIds, tokenId);
    });

    it('balancesWithProof() fetches identity balances with proof', async () => {
      const identityIds = [identityId];

      await client.tokens.balancesWithProof(identityIds, tokenId);

      expect(wasmSdk.getIdentitiesTokenBalancesWithProofInfo).to.be.calledOnceWithExactly(identityIds, tokenId);
    });

    it('identityBalances() fetches balances for multiple tokens of one identity', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityBalances(identityId, tokenIds);

      expect(wasmSdk.getIdentityTokenBalances).to.be.calledOnceWithExactly(identityId, tokenIds);
    });

    it('identityBalancesWithProof() fetches identity token balances with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityBalancesWithProof(identityId, tokenIds);

      expect(wasmSdk.getIdentityTokenBalancesWithProofInfo).to.be.calledOnceWithExactly(identityId, tokenIds);
    });

    it('identityTokenInfos() fetches token info for an identity', async () => {
      const tokenIds = [tokenId, 'AnotherTokenId123456789abcdefghijklmnop'];

      await client.tokens.identityTokenInfos(identityId, tokenIds);

      expect(wasmSdk.getIdentityTokenInfos).to.be.calledOnceWithExactly(identityId, tokenIds);
    });

    it('identitiesTokenInfos() fetches token info for multiple identities', async () => {
      const identityIds = [identityId];

      await client.tokens.identitiesTokenInfos(identityIds, tokenId);

      expect(wasmSdk.getIdentitiesTokenInfos).to.be.calledOnceWithExactly(identityIds, tokenId);
    });

    it('identityTokenInfosWithProof() fetches token info with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.identityTokenInfosWithProof(identityId, tokenIds);

      expect(wasmSdk.getIdentityTokenInfosWithProofInfo).to.be.calledOnceWithExactly(identityId, tokenIds);
    });

    it('identitiesTokenInfosWithProof() fetches multiple identities info with proof', async () => {
      const identityIds = [identityId];

      await client.tokens.identitiesTokenInfosWithProof(identityIds, tokenId);

      expect(wasmSdk.getIdentitiesTokenInfosWithProofInfo).to.be.calledOnceWithExactly(identityIds, tokenId);
    });

    it('directPurchasePrices() fetches purchase prices for tokens', async () => {
      const tokenIds = [tokenId];

      await client.tokens.directPurchasePrices(tokenIds);

      expect(wasmSdk.getTokenDirectPurchasePrices).to.be.calledOnceWithExactly(tokenIds);
    });

    it('directPurchasePricesWithProof() fetches purchase prices with proof', async () => {
      const tokenIds = [tokenId];

      await client.tokens.directPurchasePricesWithProof(tokenIds);

      expect(wasmSdk.getTokenDirectPurchasePricesWithProofInfo).to.be.calledOnceWithExactly(tokenIds);
    });

    it('contractInfo() fetches token contract information', async () => {
      await client.tokens.contractInfo(contractId);

      expect(wasmSdk.getTokenContractInfo).to.be.calledOnceWithExactly(contractId);
    });

    it('contractInfoWithProof() fetches contract info with proof', async () => {
      await client.tokens.contractInfoWithProof(contractId);

      expect(wasmSdk.getTokenContractInfoWithProofInfo).to.be.calledOnceWithExactly(contractId);
    });

    it('perpetualDistributionLastClaim() fetches last claim time', async () => {
      await client.tokens.perpetualDistributionLastClaim(identityId, tokenId);

      expect(wasmSdk.getTokenPerpetualDistributionLastClaim).to.be.calledOnceWithExactly(identityId, tokenId);
    });

    it('perpetualDistributionLastClaimWithProof() fetches last claim with proof', async () => {
      await client.tokens.perpetualDistributionLastClaimWithProof(identityId, tokenId);

      expect(wasmSdk.getTokenPerpetualDistributionLastClaimWithProofInfo).to.be.calledOnceWithExactly(identityId, tokenId);
    });
  });

  describe('Transition Methods', () => {
    it('mint() mints new tokens to an identity', async () => {
      const options = {
        tokenId,
        amount: BigInt(50000000), // 50M tokens
        recipientId,
        identityKey,
        signer,
        publicNote: 'Initial token distribution',
      };

      const result = await client.tokens.mint(options);

      expect(wasmSdk.tokenMint).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(100000000));
    });

    it('burn() burns tokens from an identity', async () => {
      const options = {
        tokenId,
        amount: BigInt(10000000), // 10M tokens
        identityKey,
        signer,
        publicNote: 'Token buyback and burn',
      };

      const result = await client.tokens.burn(options);

      expect(wasmSdk.tokenBurn).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(50000000));
    });

    it('transfer() transfers tokens between identities', async () => {
      const options = {
        tokenId,
        amount: BigInt(25000000), // 25M tokens
        recipientId,
        identityKey,
        signer,
        publicNote: 'Payment for services',
      };

      const result = await client.tokens.transfer(options);

      expect(wasmSdk.tokenTransfer).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.senderBalance).to.equal(BigInt(40000000));
      expect(result.recipientBalance).to.equal(BigInt(60000000));
    });

    it('freeze() freezes tokens for an identity', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Account frozen for compliance review',
      };

      const result = await client.tokens.freeze(options);

      expect(wasmSdk.tokenFreeze).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });

    it('unfreeze() unfreezes previously frozen tokens', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Compliance review completed',
      };

      const result = await client.tokens.unfreeze(options);

      expect(wasmSdk.tokenUnfreeze).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });

    it('destroyFrozen() destroys frozen tokens', async () => {
      const frozenIdentityId = recipientId;
      const options = {
        tokenId,
        frozenIdentityId,
        identityKey,
        signer,
        publicNote: 'Fraudulent tokens destroyed',
      };

      const result = await client.tokens.destroyFrozen(options);

      expect(wasmSdk.tokenDestroyFrozen).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });

    it('emergencyAction() executes emergency token action', async () => {
      const options = {
        tokenId,
        action: 'pause',
        identityKey,
        signer,
        publicNote: 'Emergency pause due to security concern',
      };

      const result = await client.tokens.emergencyAction(options);

      expect(wasmSdk.tokenEmergencyAction).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });

    it('setPrice() sets direct purchase price for tokens', async () => {
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

      expect(wasmSdk.tokenSetPrice).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
    });

    it('directPurchase() purchases tokens directly', async () => {
      const options = {
        tokenId,
        amount: BigInt(5000000), // 5M tokens
        totalAgreedPrice: BigInt(5000000000), // 5B credits
        identityKey,
        signer,
      };

      const result = await client.tokens.directPurchase(options);

      expect(wasmSdk.tokenDirectPurchase).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.balance).to.equal(BigInt(10000000));
    });

    it('claim() claims token distribution rewards', async () => {
      const options = {
        tokenId,
        identityKey,
        signer,
        publicNote: 'Claiming weekly distribution',
      };

      const result = await client.tokens.claim(options);

      expect(wasmSdk.tokenClaim).to.be.calledOnceWithExactly(options);
      expect(result.tokenId).to.equal(tokenId);
      expect(result.claimedAmount).to.equal(BigInt(5000000));
    });
  });
});
