import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/local.mjs';

describe('Documents', function documentsSuite() {
  this.timeout(90000);
  let sdk;
  let documentId;

  before(async () => {
    sdk = EvoSDK.localTrusted();
    await sdk.connect();

    const res = await sdk.documents.query({
      dataContractId: TEST_IDS.dataContractId,
      documentTypeName: TEST_IDS.documentType,
      limit: 1,
    });
    const first = res?.get(TEST_IDS.documentType)?.[0];
    documentId = first?.getId?.()?.toString?.() || TEST_IDS.documentId;
  });

  it('query() returns documents by type', async () => {
    const res = await sdk.documents.query({
      dataContractId: TEST_IDS.dataContractId,
      documentTypeName: TEST_IDS.documentType,
      limit: 5,
      orderBy: [['normalizedLabel', 'desc']],
    });
    expect(res).to.exist();
  });

  it('get() returns a single document by id', async () => {
    const res = await sdk.documents.get(
      TEST_IDS.dataContractId,
      TEST_IDS.documentType,
      documentId,
    );
    expect(res).to.exist();
  });
});
