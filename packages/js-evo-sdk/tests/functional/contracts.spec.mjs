import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Data Contracts', function dataContractsSuite() {
  let sdk;

  this.timeout(60000);

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('fetch() returns data contract', async () => {
    const res = await sdk.contracts.fetch(TEST_IDS.dataContractId);
    expect(res).to.exist();
  });

  it('fetchWithProof() returns proof info', async () => {
    const res = await sdk.contracts.fetchWithProof(TEST_IDS.dataContractId);
    expect(res).to.exist();
  });

  // TODO: fix dash drive: proof: corrupted error: we did not get back an element
  //  for the correct path for the historical contract
  it.skip('getHistory() returns history for contract', async () => {
    const res = await sdk.contracts.getHistory({ contractId: TEST_IDS.tokenContractId, limit: 1 });
    expect(res).to.exist();
  });

  it('getMany() returns multiple contracts', async () => {
    const res = await sdk.contracts.getMany([TEST_IDS.dataContractId]);
    expect(res).to.exist();
  });
});
