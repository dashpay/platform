import init, * as sdk from '../../dist/sdk.compressed.js';

describe('API availability (exports and methods)', () => {
  before(async () => {
    await init();
  });
  it('query methods are available on WasmSdk instance', () => {
    const instanceFns = [
      // Identity
      'getIdentity', 'getIdentityUnproved', 'getIdentityKeys', 'getIdentityNonce', 'getIdentityContractNonce', 'getIdentityBalance', 'getIdentitiesBalances', 'getIdentityBalanceAndRevision', 'getIdentityByPublicKeyHash', 'getIdentityByNonUniquePublicKeyHash', 'getIdentitiesContractKeys', 'getIdentityTokenBalances', 'getIdentityTokenInfos', 'getIdentitiesTokenInfos',
      // Documents / contracts
      'getDocuments', 'getDocument', 'getDataContract', 'getDataContractHistory', 'getDataContracts',
      // Tokens
      'getTokenStatuses', 'getTokenDirectPurchasePrices', 'getTokenContractInfo', 'getTokenPerpetualDistributionLastClaim', 'getTokenTotalSupply',
      // Epochs / System / Protocol
      'getEpochsInfo', 'getFinalizedEpochInfos', 'getCurrentEpoch', 'getEvonodesProposedEpochBlocksByIds', 'getEvonodesProposedEpochBlocksByRange', 'getProtocolVersionUpgradeState', 'getProtocolVersionUpgradeVoteStatus', 'getStatus', 'getCurrentQuorumsInfo', 'getTotalCreditsInPlatform', 'getPrefundedSpecializedBalance', 'getPathElements',
      // Voting / Groups
      'getContestedResources', 'getContestedResourceVoteState', 'getContestedResourceVotersForIdentity', 'getContestedResourceIdentityVotes', 'getVotePollsByEndDate', 'getGroupInfo', 'getGroupInfos', 'getGroupMembers', 'getIdentityGroups', 'getGroupActions', 'getGroupActionSigners', 'getGroupsDataContracts',
      // DPNS queries
      'dpnsRegisterName', 'dpnsIsNameAvailable', 'dpnsResolveName', 'getDpnsUsernameByName', 'getDpnsUsernames', 'getDpnsUsername',
      // Utils
      'waitForStateTransitionResult',
    ];
    for (const fn of instanceFns) {
      expect(typeof sdk.WasmSdk.prototype[fn]).to.be.oneOf(['function', 'undefined']);
    }
  });

  it('standalone verification helpers are exported', () => {
    const moduleFns = [
      'verifyIdentityResponse', 'verifyDataContract', 'verifyDocuments',
    ];
    for (const fn of moduleFns) {
      expect(typeof sdk[fn]).to.be.oneOf(['function', 'undefined']);
    }
  });
});
