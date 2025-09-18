import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Tokens', function tokensSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('totalSupply() returns supply for token', async () => {
    const res = await sdk.tokens.totalSupply(TEST_IDS.tokenId);
    expect(res).to.exist();
  });

  it('statuses() returns statuses for token(s)', async () => {
    const res = await sdk.tokens.statuses([TEST_IDS.tokenId]);
    expect(res).to.exist();
  });

  it('directPurchasePrices() returns prices for token(s)', async () => {
    const res = await sdk.tokens.directPurchasePrices([TEST_IDS.tokenId]);
    expect(res).to.exist();
  });

  // TODO: Fix this test
  it.skip('contractInfo() returns token contract info', async () => {
    const res = await sdk.tokens.contractInfo(TEST_IDS.tokenContractId);
    expect(res).to.exist();
  });

  it('identitiesTokenInfos() returns token infos for identities', async () => {
    const res = await sdk.tokens.identitiesTokenInfos([TEST_IDS.identityId], TEST_IDS.tokenId);
    expect(res).to.exist();
  });
});
