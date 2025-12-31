import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('TokensFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // query methods
    this.sinon.stub(wasmSdk, 'getTokenPriceByContract').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenTotalSupply').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenTotalSupplyWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenStatuses').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenStatusesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalances').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenBalancesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalances').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenBalancesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenInfos').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfos').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityTokenInfosWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentitiesTokenInfosWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePrices').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenDirectPurchasePricesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenContractInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenContractInfoWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaim').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTokenPerpetualDistributionLastClaimWithProofInfo').resolves('ok');

    // tx methods - all return result objects
    this.sinon.stub(wasmSdk, 'tokenMint').resolves({ tokenId: 't', balance: BigInt(100) });
    this.sinon.stub(wasmSdk, 'tokenBurn').resolves({ tokenId: 't', balance: BigInt(50) });
    this.sinon.stub(wasmSdk, 'tokenTransfer').resolves({ tokenId: 't', senderBalance: BigInt(40), recipientBalance: BigInt(60) });
    this.sinon.stub(wasmSdk, 'tokenFreeze').resolves({ tokenId: 't' });
    this.sinon.stub(wasmSdk, 'tokenUnfreeze').resolves({ tokenId: 't' });
    this.sinon.stub(wasmSdk, 'tokenDestroyFrozen').resolves({ tokenId: 't' });
    this.sinon.stub(wasmSdk, 'tokenEmergencyAction').resolves({ tokenId: 't' });
    this.sinon.stub(wasmSdk, 'tokenSetPrice').resolves({ tokenId: 't' });
    this.sinon.stub(wasmSdk, 'tokenDirectPurchase').resolves({ tokenId: 't', balance: BigInt(10) });
    this.sinon.stub(wasmSdk, 'tokenClaim').resolves({ tokenId: 't', claimedAmount: BigInt(5) });
  });

  it('calculateId() uses wasm static helper', async () => {
    const out = await client.tokens.calculateId('Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv', 0);
    expect(out).to.equal('BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus');
  });

  it('query functions forward to instance methods', async () => {
    await client.tokens.priceByContract('c', 1);
    await client.tokens.totalSupply('t');
    await client.tokens.totalSupplyWithProof('t');
    await client.tokens.statuses(['a', 'b']);
    await client.tokens.statusesWithProof(['a']);
    await client.tokens.balances(['i1', 'i2'], 't');
    await client.tokens.balancesWithProof(['i'], 't');
    await client.tokens.identityBalances('id', ['t']);
    await client.tokens.identityBalancesWithProof('id', ['t']);
    await client.tokens.identityTokenInfos('id', ['t1', 't2']);
    await client.tokens.identitiesTokenInfos(['i1'], 't');
    await client.tokens.identityTokenInfosWithProof('id', ['t']);
    await client.tokens.identitiesTokenInfosWithProof(['i'], 't');
    await client.tokens.directPurchasePrices(['t']);
    await client.tokens.directPurchasePricesWithProof(['t']);
    await client.tokens.contractInfo('c');
    await client.tokens.contractInfoWithProof('c');
    await client.tokens.perpetualDistributionLastClaim('i', 't');
    await client.tokens.perpetualDistributionLastClaimWithProof('i', 't');
    expect(wasmSdk.getTokenPriceByContract).to.be.calledOnceWithExactly('c', 1);
    expect(wasmSdk.getTokenTotalSupply).to.be.calledOnceWithExactly('t');
    expect(wasmSdk.getTokenTotalSupplyWithProofInfo).to.be.calledOnceWithExactly('t');
    expect(wasmSdk.getTokenStatuses).to.be.calledOnceWithExactly(['a', 'b']);
    expect(wasmSdk.getTokenStatusesWithProofInfo).to.be.calledOnceWithExactly(['a']);
    expect(wasmSdk.getIdentitiesTokenBalances).to.be.calledOnceWithExactly(['i1', 'i2'], 't');
    expect(wasmSdk.getIdentitiesTokenBalancesWithProofInfo).to.be.calledOnceWithExactly(['i'], 't');
    expect(wasmSdk.getIdentityTokenBalances).to.be.calledOnceWithExactly('id', ['t']);
    expect(wasmSdk.getIdentityTokenBalancesWithProofInfo).to.be.calledOnceWithExactly('id', ['t']);
    expect(wasmSdk.getIdentityTokenInfos).to.be.calledOnceWithExactly('id', ['t1', 't2']);
    expect(wasmSdk.getIdentitiesTokenInfos).to.be.calledOnceWithExactly(['i1'], 't');
    expect(wasmSdk.getIdentityTokenInfosWithProofInfo).to.be.calledOnceWithExactly('id', ['t']);
    expect(wasmSdk.getIdentitiesTokenInfosWithProofInfo).to.be.calledOnceWithExactly(['i'], 't');
    expect(wasmSdk.getTokenDirectPurchasePrices).to.be.calledOnceWithExactly(['t']);
    expect(wasmSdk.getTokenDirectPurchasePricesWithProofInfo).to.be.calledOnceWithExactly(['t']);
    expect(wasmSdk.getTokenContractInfo).to.be.calledOnceWithExactly('c');
    expect(wasmSdk.getTokenContractInfoWithProofInfo).to.be.calledOnceWithExactly('c');
    expect(wasmSdk.getTokenPerpetualDistributionLastClaim).to.be.calledOnceWithExactly('i', 't');
    expect(wasmSdk.getTokenPerpetualDistributionLastClaimWithProofInfo).to.be.calledOnceWithExactly('i', 't');
  });

  it('mint() calls wasmSdk.tokenMint with options', async () => {
    const options = {
      tokenId: 't',
      amount: BigInt(3),
      signer: { privateKeyWif: 'w' },
      recipientId: 'r',
      publicNote: 'n',
    };
    await client.tokens.mint(options);
    expect(wasmSdk.tokenMint).to.be.calledOnceWithExactly(options);
  });

  it('burn() calls wasmSdk.tokenBurn with options', async () => {
    const options = {
      tokenId: 't',
      amount: BigInt(5),
      signer: { privateKeyWif: 'w' },
    };
    await client.tokens.burn(options);
    expect(wasmSdk.tokenBurn).to.be.calledOnceWithExactly(options);
  });

  it('transfer() calls wasmSdk.tokenTransfer with options', async () => {
    const options = {
      tokenId: 't',
      amount: BigInt(7),
      recipientId: 'r',
      signer: { privateKeyWif: 'w' },
      publicNote: 'n',
    };
    await client.tokens.transfer(options);
    expect(wasmSdk.tokenTransfer).to.be.calledOnceWithExactly(options);
  });

  it('freeze() calls wasmSdk.tokenFreeze with options', async () => {
    const options = {
      tokenId: 't',
      frozenIdentityId: 'i',
      signer: { privateKeyWif: 'w' },
      publicNote: 'n',
    };
    await client.tokens.freeze(options);
    expect(wasmSdk.tokenFreeze).to.be.calledOnceWithExactly(options);
  });

  it('unfreeze() calls wasmSdk.tokenUnfreeze with options', async () => {
    const options = {
      tokenId: 't',
      frozenIdentityId: 'i',
      signer: { privateKeyWif: 'w' },
      publicNote: 'n',
    };
    await client.tokens.unfreeze(options);
    expect(wasmSdk.tokenUnfreeze).to.be.calledOnceWithExactly(options);
  });

  it('destroyFrozen() calls wasmSdk.tokenDestroyFrozen with options', async () => {
    const options = {
      tokenId: 't',
      frozenIdentityId: 'i',
      signer: { privateKeyWif: 'w' },
    };
    await client.tokens.destroyFrozen(options);
    expect(wasmSdk.tokenDestroyFrozen).to.be.calledOnceWithExactly(options);
  });

  it('emergencyAction() calls wasmSdk.tokenEmergencyAction with options', async () => {
    const options = {
      tokenId: 't',
      action: 'pause',
      signer: { privateKeyWif: 'w' },
    };
    await client.tokens.emergencyAction(options);
    expect(wasmSdk.tokenEmergencyAction).to.be.calledOnceWithExactly(options);
  });

  it('setPrice() calls wasmSdk.tokenSetPrice with options', async () => {
    const options = {
      tokenId: 't',
      price: { type: 'fixed', value: BigInt(100) },
      signer: { privateKeyWif: 'w' },
    };
    await client.tokens.setPrice(options);
    expect(wasmSdk.tokenSetPrice).to.be.calledOnceWithExactly(options);
  });

  it('directPurchase() calls wasmSdk.tokenDirectPurchase with options', async () => {
    const options = {
      tokenId: 't',
      amount: BigInt(2),
      totalAgreedPrice: BigInt(10),
      signer: { privateKeyWif: 'w' },
    };
    await client.tokens.directPurchase(options);
    expect(wasmSdk.tokenDirectPurchase).to.be.calledOnceWithExactly(options);
  });

  it('claim() calls wasmSdk.tokenClaim with options', async () => {
    const options = {
      tokenId: 't',
      signer: { privateKeyWif: 'w' },
      publicNote: 'n',
    };
    await client.tokens.claim(options);
    expect(wasmSdk.tokenClaim).to.be.calledOnceWithExactly(options);
  });
});
