const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Feature flags', function main() {
    this.timeout(900000);

    describe('updateConsensusParams', () => {
      let oldConsensusParams;
      let ownerClient;
      let updateConsensusParamsFeatureFlag;
      let revertConsensusParamsFeatureFlag;
      let identity;

      before(async () => {
        ownerClient = await createClientWithFundedWallet(
          process.env.DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY,
        );

        const featureFlagContract = await ownerClient.platform.contracts.get(
          process.env.FEATURE_FLAGS_CONTRACT_ID,
        );

        ownerClient.getApps().set('featureFlags', {
          contractId: process.env.FEATURE_FLAGS_CONTRACT_ID,
          contract: featureFlagContract,
        });

        identity = await ownerClient.platform.identities.get(
          process.env.FEATURE_FLAGS_IDENTITY_ID,
        );

        const { blockHeight: lastBlockHeight } = identity.getMetadata();

        oldConsensusParams = await ownerClient.getDAPIClient().platform.getConsensusParams();

        const block = oldConsensusParams.getBlock();
        const evidence = oldConsensusParams.getEvidence();

        updateConsensusParamsFeatureFlag = {
          enableAtHeight: lastBlockHeight + 2,
          block: {
            maxBytes: +block.maxBytes + 1,
          },
          evidence: {
            maxAgeNumBlocks: +evidence.maxAgeNumBlocks + 1,
            maxAgeDuration: {
              seconds: Math.trunc(evidence.maxAgeDuration / 1000000000) + 1,
              nanos: (evidence.maxAgeDuration % 1000000000) + 1,
            },
            // on schnaps, maxBytes default value is empty, and we can't revert it back
            // maxBytes: +evidence.maxBytes + 1,
          },
        };

        revertConsensusParamsFeatureFlag = {
          enableAtHeight: lastBlockHeight + 4,
          block: {
            maxBytes: +block.maxBytes,
          },
          evidence: {
            maxAgeNumBlocks: +evidence.maxAgeNumBlocks,
            maxAgeDuration: {
              seconds: Math.trunc(evidence.maxAgeDuration / 1000000000),
              nanos: (evidence.maxAgeDuration % 1000000000),
            },
            // maxBytes: +evidence.maxBytes,
          },
        };
      });

      it('should update consensus params', async function it() {
        if (process.env.NETWORK === 'mainnet') {
          this.skip('it\'s dangerous to run this test on mainnet');
        }

        const documentUpdate = await ownerClient.platform.documents.create(
          'featureFlags.updateConsensusParams',
          identity,
          updateConsensusParamsFeatureFlag,
        );

        const documentRevert = await ownerClient.platform.documents.create(
          'featureFlags.updateConsensusParams',
          identity,
          revertConsensusParamsFeatureFlag,
        );

        await ownerClient.platform.documents.broadcast({
          create: [documentUpdate, documentRevert],
        }, identity);

        // wait for block and check consensus params were changed
        let height;
        do {
          const someIdentity = await ownerClient.platform.identities.get(
            process.env.FEATURE_FLAGS_IDENTITY_ID,
          );

          ({ blockHeight: height } = someIdentity.getMetadata());
        } while (height <= updateConsensusParamsFeatureFlag.enableAtHeight);

        let newConsensusParams = await ownerClient.getDAPIClient().platform.getConsensusParams(
          updateConsensusParamsFeatureFlag.enableAtHeight + 1,
        );

        const { block, evidence } = updateConsensusParamsFeatureFlag;

        let updatedBlock = newConsensusParams.getBlock();

        expect(updatedBlock.getMaxBytes()).to.equal(`${block.maxBytes}`);

        const { seconds } = evidence.maxAgeDuration;
        const nanos = `${evidence.maxAgeDuration.nanos}`.padStart(9, '0');

        let updatedEvidence = newConsensusParams.getEvidence();

        expect(updatedEvidence.getMaxAgeNumBlocks()).to.equal(`${evidence.maxAgeNumBlocks}`);
        expect(updatedEvidence.getMaxAgeDuration()).to.equal(`${seconds}${nanos}`);
        // expect(updatedEvidence.getMaxBytes()).to.equal(`${evidence.maxBytes}`);

        // wait for block and check consensus params were reverted
        do {
          const someIdentity = await ownerClient.platform.identities.get(
            process.env.FEATURE_FLAGS_IDENTITY_ID,
          );

          ({ blockHeight: height } = someIdentity.getMetadata());
        } while (height <= revertConsensusParamsFeatureFlag.enableAtHeight);

        newConsensusParams = await ownerClient.getDAPIClient().platform.getConsensusParams(
          revertConsensusParamsFeatureFlag.enableAtHeight + 1,
        );

        updatedBlock = newConsensusParams.getBlock();
        const oldBlock = oldConsensusParams.getBlock();

        expect(updatedBlock.getMaxBytes()).to.equal(`${oldBlock.maxBytes}`);

        updatedEvidence = newConsensusParams.getEvidence();
        const oldEvidence = oldConsensusParams.getEvidence();

        expect(updatedEvidence.getMaxAgeNumBlocks()).to.equal(`${oldEvidence.maxAgeNumBlocks}`);
        expect(updatedEvidence.getMaxAgeDuration()).to.equal(`${oldEvidence.maxAgeDuration}`);
        // expect(updatedEvidence.getMaxBytes()).to.equal(`${oldEvidence.maxBytes}`);
      });
    });
  });
});
