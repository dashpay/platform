const featureFlagsSystemIds = require('@dashevo/feature-flags-contract/lib/systemIds');
const InvalidRequestError = require('@dashevo/dapi-client/lib/transport/errors/response/InvalidRequestError');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

// TODO(rs-drive-abci): restore.
//   Some consensus params in assertions do not match anymore.
//   Needs to be checeked if it's a bug or intended behaviour
//   at rs-drive-abci side
describe.skip('Platform', () => {
  describe('Feature flags', function main() {
    // this.timeout(900000);

    describe('updateConsensusParams', () => {
      let oldConsensusParams;
      let ownerClient;
      let updateConsensusParamsFeatureFlag;
      let revertConsensusParamsFeatureFlag;
      let identity;

      let contractId;
      let ownerId;

      before(async () => {
        ownerClient = await createClientWithFundedWallet(
          450000,
          process.env.FEATURE_FLAGS_OWNER_PRIVATE_KEY,
        );

        await ownerClient.platform.identities.topUp(featureFlagsSystemIds.ownerId, 250000);

        ({ contractId, ownerId } = featureFlagsSystemIds);

        const featureFlagContract = await ownerClient.platform.contracts.get(
          contractId,
        );

        ownerClient.getApps().set('featureFlags', {
          contractId,
          contract: featureFlagContract,
        });

        identity = await ownerClient.platform.identities.get(
          ownerId,
        );

        const metadata = identity.getMetadata();

        const lastBlockHeight = metadata.getBlockHeight();

        oldConsensusParams = await ownerClient.getDAPIClient().platform.getConsensusParams();

        const block = oldConsensusParams.getBlock();
        const evidence = oldConsensusParams.getEvidence();

        updateConsensusParamsFeatureFlag = {
          enableAtHeight: lastBlockHeight + 3,
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
          enableAtHeight: lastBlockHeight + 5,
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
          create: [documentUpdate],
        }, identity);

        // forcing creation of additional block (enableAtHeight + 1)
        await ownerClient.platform.documents.broadcast({
          create: [documentRevert],
        }, identity);

        // forcing creation of additional block (enableAtHeight + 2)
        await ownerClient.platform.identities.register(100000);

        // wait for block and check consensus params were changed
        let height;
        do {
          const someIdentity = await ownerClient.platform.identities.get(
            ownerId,
          );

          const metadata = someIdentity.getMetadata();
          height = metadata.getBlockHeight();
        } while (height <= updateConsensusParamsFeatureFlag.enableAtHeight);

        let newConsensusParams;

        for (let i = 0; i < 5; i++) {
          try {
            newConsensusParams = await ownerClient.getDAPIClient()
              .platform
              .getConsensusParams(
                updateConsensusParamsFeatureFlag.enableAtHeight + 1,
              );
          } catch (e) {
            if (!(e instanceof InvalidRequestError) || !e.message.startsWith('Invalid height') || i + 1 === 5) {
              throw e;
            }
          }
        }

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
            ownerId,
          );

          const metadata = someIdentity.getMetadata();
          height = metadata.getBlockHeight();
        } while (height <= revertConsensusParamsFeatureFlag.enableAtHeight);

        for (let i = 0; i < 5; i++) {
          try {
            newConsensusParams = await ownerClient.getDAPIClient()
              .platform
              .getConsensusParams(
                revertConsensusParamsFeatureFlag.enableAtHeight + 1,
              );
          } catch (e) {
            if (!(e instanceof InvalidRequestError) || !e.message.startsWith('Invalid height') || i + 1 === 5) {
              throw e;
            }
          }
        }

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
