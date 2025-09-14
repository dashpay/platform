import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import sinon from 'sinon';

const isBrowser = typeof window !== 'undefined';

describe('TokensFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('query functions forward to free functions', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.tokens.priceByContract('c', 1);
    await sdk.tokens.totalSupply('t');
    await sdk.tokens.totalSupplyWithProof('t');
    await sdk.tokens.statuses(['a', 'b']);
    await sdk.tokens.statusesWithProof(['a']);
    await sdk.tokens.balances(['i1', 'i2'], 't');
    await sdk.tokens.balancesWithProof(['i'], 't');
    await sdk.tokens.identityTokenInfos('id', ['t1', 't2']);
    await sdk.tokens.identitiesTokenInfos(['i1'], 't');
    await sdk.tokens.identityTokenInfosWithProof('id', ['t']);
    await sdk.tokens.identitiesTokenInfosWithProof(['i'], 't');
    await sdk.tokens.directPurchasePrices(['t']);
    await sdk.tokens.directPurchasePricesWithProof(['t']);
    await sdk.tokens.contractInfo('c');
    await sdk.tokens.contractInfoWithProof('c');
    await sdk.tokens.perpetualDistributionLastClaim('i', 't');
    await sdk.tokens.perpetualDistributionLastClaimWithProof('i', 't');

    const calls = wasmStubModule.__getCalls();
    expect(calls.map(c => c.called)).to.include.members([
      'get_token_price_by_contract',
      'get_token_total_supply',
      'get_token_total_supply_with_proof_info',
      'get_token_statuses',
      'get_token_statuses_with_proof_info',
      'get_identities_token_balances',
      'get_identities_token_balances_with_proof_info',
      'get_identity_token_infos',
      'get_identities_token_infos',
      'get_identity_token_infos_with_proof_info',
      'get_identities_token_infos_with_proof_info',
      'get_token_direct_purchase_prices',
      'get_token_direct_purchase_prices_with_proof_info',
      'get_token_contract_info',
      'get_token_contract_info_with_proof_info',
      'get_token_perpetual_distribution_last_claim',
      'get_token_perpetual_distribution_last_claim_with_proof_info',
    ]);
    // Spot-check one
    const first = calls.find(c => c.called === 'get_token_price_by_contract');
    expect(first.args).to.deep.equal([raw, 'c', 1]);
  });

  it('mint() stringifies amount and passes nullables', async () => {
    const wasm = { tokenMint: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.mint({ contractId: 'c', tokenPosition: 1, amount: 3n, identityId: 'i', privateKeyWif: 'w', recipientId: 'r', publicNote: 'n' });
    sinon.assert.calledOnceWithExactly(wasm.tokenMint, 'c', 1, '3', 'i', 'w', 'r', 'n');
  });

  it('burn() stringifies amount', async () => {
    const wasm = { tokenBurn: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.burn({ contractId: 'c', tokenPosition: 1, amount: 5, identityId: 'i', privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.tokenBurn, 'c', 1, '5', 'i', 'w', null);
  });

  it('transfer() stringifies amount and forwards', async () => {
    const wasm = { tokenTransfer: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.transfer({ contractId: 'c', tokenPosition: 2, amount: '7', senderId: 's', recipientId: 'r', privateKeyWif: 'w', publicNote: 'n' });
    sinon.assert.calledOnceWithExactly(wasm.tokenTransfer, 'c', 2, '7', 's', 'r', 'w', 'n');
  });

  it('freeze()/unfreeze()/destroyFrozen() pass through args', async () => {
    const wasm = {
      tokenFreeze: sinon.stub().resolves('ok'),
      tokenUnfreeze: sinon.stub().resolves('ok'),
      tokenDestroyFrozen: sinon.stub().resolves('ok'),
    };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.freeze({ contractId: 'c', tokenPosition: 1, identityToFreeze: 'i', freezerId: 'f', privateKeyWif: 'w', publicNote: 'n' });
    await sdk.tokens.unfreeze({ contractId: 'c', tokenPosition: 1, identityToUnfreeze: 'i', unfreezerId: 'u', privateKeyWif: 'w', publicNote: 'n2' });
    await sdk.tokens.destroyFrozen({ contractId: 'c', tokenPosition: 1, identityId: 'i', destroyerId: 'd', privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.tokenFreeze, 'c', 1, 'i', 'f', 'w', 'n');
    sinon.assert.calledOnceWithExactly(wasm.tokenUnfreeze, 'c', 1, 'i', 'u', 'w', 'n2');
    sinon.assert.calledOnceWithExactly(wasm.tokenDestroyFrozen, 'c', 1, 'i', 'd', 'w', null);
  });

  it('setPriceForDirectPurchase() JSON stringifies priceData', async () => {
    const wasm = { tokenSetPriceForDirectPurchase: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.setPriceForDirectPurchase({ contractId: 'c', tokenPosition: 1, identityId: 'i', priceType: 't', priceData: { p: 1 }, privateKeyWif: 'w', publicNote: 'n' });
    sinon.assert.calledOnceWithExactly(wasm.tokenSetPriceForDirectPurchase, 'c', 1, 'i', 't', JSON.stringify({ p: 1 }), 'w', 'n');
  });

  it('directPurchase() stringifies amount and totalAgreedPrice', async () => {
    const wasm = { tokenDirectPurchase: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.directPurchase({ contractId: 'c', tokenPosition: 1, amount: 2, identityId: 'i', totalAgreedPrice: 10, privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.tokenDirectPurchase, 'c', 1, '2', 'i', '10', 'w');
  });

  it('claim() and configUpdate() forward with JSON where needed', async () => {
    const wasm = { tokenClaim: sinon.stub().resolves('ok'), tokenConfigUpdate: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.tokens.claim({ contractId: 'c', tokenPosition: 1, distributionType: 'd', identityId: 'i', privateKeyWif: 'w', publicNote: 'n' });
    await sdk.tokens.configUpdate({ contractId: 'c', tokenPosition: 1, configItemType: 't', configValue: { v: true }, identityId: 'i', privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.tokenClaim, 'c', 1, 'd', 'i', 'w', 'n');
    sinon.assert.calledOnceWithExactly(wasm.tokenConfigUpdate, 'c', 1, 't', JSON.stringify({ v: true }), 'i', 'w', null);
  });
});

