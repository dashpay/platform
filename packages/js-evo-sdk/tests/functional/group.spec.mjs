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

  it('info() is callable for group contracts', async () => {
    const res = await sdk.group.info(TEST_IDS.groupContractId, 0);
    expect(res).to.exist();
  });

  it('members() is callable for group contracts', async () => {
    const res = await sdk.group.members(TEST_IDS.groupContractId, 0, { limit: 5 });
    expect(res).to.exist();
  });

  it('identityGroups() is callable for known identity', async () => {
    const res = await sdk.group.identityGroups(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('actions() is callable for group contract', async () => {
    const res = await sdk.group.actions(TEST_IDS.groupContractId, 0, 'ACTIVE', { count: 5 });
    expect(res).to.exist();
  });

  it('groupsDataContracts() is callable with known ids', async () => {
    const res = await sdk.group.groupsDataContracts([TEST_IDS.groupContractId]);
    expect(res).to.not.equal(undefined);
  });
});
