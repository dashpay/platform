import { expect } from 'chai';
import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Group', function groupSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('contestedResources() returns contested resources (may be empty)', async () => {
    const res = await sdk.group.contestedResources({
      documentTypeName: 'domain',
      contractId: TEST_IDS.dataContractId,
      indexName: 'parentNameAndLabel',
      limit: 5,
    });
    expect(res).to.exist();
  });

  it('contestedResourceVotersForIdentity() returns voters (may be empty)', async () => {
    const res = await sdk.group.contestedResourceVotersForIdentity({
      contractId: TEST_IDS.dataContractId,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues: ['dash', TEST_IDS.username],
      contestantId: TEST_IDS.identityId,
      limit: 5,
    });
    expect(res).to.exist();
  });
});
