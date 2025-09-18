import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Voting', function votingSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('contestedResourceVoteState() returns a vote state (may be empty)', async () => {
    const res = await sdk.voting.contestedResourceVoteState({
      contractId: TEST_IDS.dataContractId,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues: ['dash', TEST_IDS.username],
      resultType: 'documents',
    });
    expect(res).to.exist();
  });

  it('contestedResourceIdentityVotes() returns votes for identity (may be empty)', async () => {
    const res = await sdk.voting.contestedResourceIdentityVotes(TEST_IDS.identityId, { limit: 5 });
    expect(res).to.exist();
  });
});
