import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Documents', function documentsSuite() {
  this.timeout(90000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('query() returns documents by type', async () => {
    const res = await sdk.documents.query({
      contractId: TEST_IDS.dataContractId,
      type: TEST_IDS.documentType,
      limit: 5,
      orderBy: [['normalizedLabel', 'desc']],
    });
    expect(res).to.exist();
  });

  it('get() returns a single document by id', async () => {
    const res = await sdk.documents.get(
      TEST_IDS.dataContractId,
      TEST_IDS.documentType,
      TEST_IDS.documentId,
    );
    expect(res).to.exist();
  });
});
