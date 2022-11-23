const moment = require('moment');

const masternodeRewardSharesSystemIds = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');

const getMasternodeRewardSharesContractFixture = require('@dashevo/dpp/lib/test/fixtures/getMasternodeRewardSharesContractFixture');
const getMasternodeRewardShareDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getMasternodeRewardShareDocumentsFixture');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('Fee Pools', () => {
  let container;
  let rsDrive;
  let mnDatas;
  let identityRepository;
  let blockInfo;

  beforeEach(async () => {
    container = await createTestDIContainer();

    blockInfo = new BlockInfo(1, 0, Date.now());

    const dataContractRepository = container.resolve('dataContractRepository');
    const documentRepository = container.resolve('documentRepository');
    identityRepository = container.resolve('identityRepository');

    /**
     * @type {Drive}
     */
    rsDrive = container.resolve('rsDrive');
    await rsDrive.getAbci().initChain({});

    // setup mn reward shares contract
    const mnSharesContract = getMasternodeRewardSharesContractFixture();
    mnSharesContract.id = Identifier.from(masternodeRewardSharesSystemIds.contractId);

    await dataContractRepository.create(mnSharesContract, blockInfo);

    mnDatas = [];
    const mnCount = 1;

    for (let i = 0; i < mnCount; i++) {
      const mnIdentity = getIdentityFixture(generateRandomIdentifier());

      const payToIdentity = getIdentityFixture(generateRandomIdentifier());

      const [payToDocument] = getMasternodeRewardShareDocumentsFixture(
        mnIdentity.getId(),
        payToIdentity.getId(),
        mnSharesContract,
      );

      await identityRepository.create(mnIdentity);
      await identityRepository.create(payToIdentity);
      await documentRepository.create(payToDocument, blockInfo);

      mnDatas.push({
        mnIdentity,
        payToIdentity,
        payToDocument,
      });
    }
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should process twenty blocks and update identity balance', async () => {
    const {
      mnIdentity,
      payToIdentity,
    } = mnDatas[0];

    const genesisTime = moment();

    let previousBlockTimeMs;
    for (let day = 1; day <= 21; day++) {
      const blockHeight = day;
      const blockTimeMs = genesisTime.clone()
        .add(day - 1, 'days')
        .valueOf();

      const blockBeginRequest = {
        blockHeight,
        blockTimeMs,
        proposerProTxHash: mnIdentity.getId(),
        validatorSetQuorumHash: Buffer.alloc(32),
      };

      if (previousBlockTimeMs) {
        blockBeginRequest.previousBlockTimeMs = previousBlockTimeMs;
      }

      previousBlockTimeMs = blockTimeMs;

      await rsDrive.getAbci().blockBegin(blockBeginRequest);

      const blockEndRequest = {
        fees: {
          processingFees: 10000,
          storageFees: 10000,
        },
      };

      await rsDrive.getAbci().blockEnd(blockEndRequest);
    }

    const fetchedMnIdentityResult = await identityRepository.fetch(mnIdentity.getId());
    const fetchedShareIdentityResult = await identityRepository.fetch(payToIdentity.getId());

    const fetchedMnIdentity = fetchedMnIdentityResult.getValue();
    const fetchedShareIdentity = fetchedShareIdentityResult.getValue();

    expect(fetchedMnIdentity.getBalance()).to.equal(180510);
    expect(fetchedShareIdentity.getBalance()).to.equal(9510);
  });
});
