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

    // tx methods
    this.sinon.stub(wasmSdk, 'tokenMint').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenBurn').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenTransfer').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenFreeze').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenUnfreeze').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenDestroyFrozen').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenSetPriceForDirectPurchase').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenDirectPurchase').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenClaim').resolves('ok');
    this.sinon.stub(wasmSdk, 'tokenConfigUpdate').resolves('ok');
  });

  it('query functions forward to instance methods', async () => {
    await client.tokens.priceByContract('c', 1);
    await client.tokens.totalSupply('t');
    await client.tokens.totalSupplyWithProof('t');
    await client.tokens.statuses(['a', 'b']);
    await client.tokens.statusesWithProof(['a']);
    await client.tokens.balances(['i1', 'i2'], 't');
    await client.tokens.balancesWithProof(['i'], 't');
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
  });

  it('mint() stringifies amount and passes nullables', async () => {
    await client.tokens.mint({
      contractId: 'c',
      tokenPosition: 1,
      amount: BigInt(3),
      identityId: 'i',
      privateKeyWif: 'w',
      recipientId: 'r',
      publicNote: 'n',
    });
    expect(wasmSdk.tokenMint).to.be.calledOnceWithExactly('c', 1, '3', 'i', 'w', 'r', 'n');
  });

  it('burn() stringifies amount', async () => {
    await client.tokens.burn({
      contractId: 'c',
      tokenPosition: 1,
      amount: 5,
      identityId: 'i',
      privateKeyWif: 'w',
    });
    expect(wasmSdk.tokenBurn).to.be.calledOnceWithExactly('c', 1, '5', 'i', 'w', null);
  });

  it('transfer() stringifies amount and forwards', async () => {
    await client.tokens.transfer({
      contractId: 'c',
      tokenPosition: 2,
      amount: '7',
      senderId: 's',
      recipientId: 'r',
      privateKeyWif: 'w',
      publicNote: 'n',
    });
    expect(wasmSdk.tokenTransfer).to.be.calledOnceWithExactly('c', 2, '7', 's', 'r', 'w', 'n');
  });

  it('freeze()/unfreeze()/destroyFrozen() pass through args', async () => {
    await client.tokens.freeze({
      contractId: 'c',
      tokenPosition: 1,
      identityToFreeze: 'i',
      freezerId: 'f',
      privateKeyWif: 'w',
      publicNote: 'n',
    });
    await client.tokens.unfreeze({
      contractId: 'c',
      tokenPosition: 1,
      identityToUnfreeze: 'i',
      unfreezerId: 'u',
      privateKeyWif: 'w',
      publicNote: 'n2',
    });
    await client.tokens.destroyFrozen({
      contractId: 'c',
      tokenPosition: 1,
      identityId: 'i',
      destroyerId: 'd',
      privateKeyWif: 'w',
    });
    expect(wasmSdk.tokenFreeze).to.be.calledOnceWithExactly('c', 1, 'i', 'f', 'w', 'n');
    expect(wasmSdk.tokenUnfreeze).to.be.calledOnceWithExactly('c', 1, 'i', 'u', 'w', 'n2');
    expect(wasmSdk.tokenDestroyFrozen).to.be.calledOnceWithExactly('c', 1, 'i', 'd', 'w', null);
  });

  it('setPriceForDirectPurchase() JSON stringifies priceData', async () => {
    await client.tokens.setPriceForDirectPurchase({
      contractId: 'c',
      tokenPosition: 1,
      identityId: 'i',
      priceType: 't',
      priceData: { p: 1 },
      privateKeyWif: 'w',
      publicNote: 'n',
    });
    expect(wasmSdk.tokenSetPriceForDirectPurchase).to.be.calledOnceWithExactly('c', 1, 'i', 't', JSON.stringify({ p: 1 }), 'w', 'n');
  });

  it('directPurchase() stringifies amount and totalAgreedPrice', async () => {
    await client.tokens.directPurchase({
      contractId: 'c',
      tokenPosition: 1,
      amount: 2,
      identityId: 'i',
      totalAgreedPrice: 10,
      privateKeyWif: 'w',
    });
    expect(wasmSdk.tokenDirectPurchase).to.be.calledOnceWithExactly('c', 1, '2', 'i', '10', 'w');
  });

  it('claim() and configUpdate() forward with JSON where needed', async () => {
    await client.tokens.claim({
      contractId: 'c',
      tokenPosition: 1,
      distributionType: 'd',
      identityId: 'i',
      privateKeyWif: 'w',
      publicNote: 'n',
    });
    await client.tokens.configUpdate({
      contractId: 'c',
      tokenPosition: 1,
      configItemType: 't',
      configValue: { v: true },
      identityId: 'i',
      privateKeyWif: 'w',
    });
    expect(wasmSdk.tokenClaim).to.be.calledOnceWithExactly('c', 1, 'd', 'i', 'w', 'n');
    expect(wasmSdk.tokenConfigUpdate).to.be.calledOnceWithExactly('c', 1, 't', JSON.stringify({ v: true }), 'i', 'w', null);
  });
});
