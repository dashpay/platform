const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const ReaderMediator = require('../../../../lib/blockchain/reader/BlockchainReaderMediator');

const Revision = require('../../../../lib/stateView/revisions/Revision');
const Reference = require('../../../../lib/stateView/revisions/Reference');
const SVContract = require('../../../../lib/stateView/contract/SVContract');
const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const getBlocksFixture = require('../../../../lib/test/fixtures/getBlocksFixture');
const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../../lib/test/fixtures/getStateTransitionsFixture');
const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');
const ReaderMediatorMock = require('../../../../lib/test/mock/BlockchainReaderMediatorMock');

const STPacketIpfsRepository = require('../../../../lib/storage/stPacket/STPacketIpfsRepository');
const addSTPacketFactory = require('../../../../lib/storage/stPacket/addSTPacketFactory');
const updateSVContractFactory = require('../../../../lib/stateView/contract/updateSVContractFactory');
const revertSVContractsForStateTransitionFactory = require('../../../../lib/stateView/contract/revertSVContractsForStateTransitionFactory');
const applyStateTransitionFactory = require('../../../../lib/stateView/applyStateTransitionFactory');
const applyStateTransitionFromReferenceFactory = require('../../../../lib/stateView/applyStateTransitionFromReferenceFactory');

describe('revertSVContractsForStateTransitionFactory', () => {
  let addSTPacket;
  let svContractMongoDbRepository;
  let applyStateTransition;
  let rpcClientMock;
  let readerMediator;
  let revertSVContractsForStateTransition;
  let mongoDb;
  let ipfsClient;

  startMongoDb().then((mongoDbInstance) => {
    mongoDb = mongoDbInstance.getDb();
  });

  startIPFS().then((ipfsInstance) => {
    ipfsClient = ipfsInstance.getApi();
  });

  beforeEach(function beforeEach() {
    const dpp = new DashPlatformProtocol({
      dataProvider: {},
    });

    const stPacketRepository = new STPacketIpfsRepository(
      ipfsClient,
      dpp,
      1000,
    );

    addSTPacket = addSTPacketFactory(stPacketRepository);

    svContractMongoDbRepository = new SVContractMongoDbRepository(mongoDb, dpp);

    const updateSVContract = updateSVContractFactory(svContractMongoDbRepository);

    readerMediator = new ReaderMediatorMock(this.sinon);

    applyStateTransition = applyStateTransitionFactory(
      stPacketRepository,
      updateSVContract,
      null,
      readerMediator,
    );

    rpcClientMock = new RpcClientMock(this.sinon);

    const applyStateTransitionFromReference = applyStateTransitionFromReferenceFactory(
      applyStateTransition,
      rpcClientMock,
    );

    revertSVContractsForStateTransition = revertSVContractsForStateTransitionFactory(
      svContractMongoDbRepository,
      rpcClientMock,
      applyStateTransition,
      applyStateTransitionFromReference,
      readerMediator,
    );
  });

  it('should remove last version of SV Contract and re-apply previous versions in order', async () => {
    // 1. Store 3 versions of DP Contracts in IPFS
    const dpContractVersions = [];

    const blocks = getBlocksFixture();
    const stateTransitions = getStateTransitionsFixture();
    const [stPacket] = getSTPacketsFixture();

    const contractId = stPacket.getDPContractId();
    const dpContract = stPacket.getDPContract();

    for (let i = 0; i < 3; i++) {
      const block = blocks[i];
      const stateTransition = stateTransitions[i];

      dpContract.setVersion(i + 1);

      await addSTPacket(stPacket);

      stateTransition.extraPayload.hashSTPacket = stPacket.hash();

      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stPacket.hash(),
        hash: dpContract.hash(),
      });

      dpContractVersions.push({
        version: (i + 1),
        block,
        stateTransition,
        stPacket,
        reference,
      });

      rpcClientMock.getRawTransaction
        .withArgs(stateTransition.hash)
        .resolves({
          result: stateTransition,
        });
    }

    // 2. Create ans store SV Contract
    const previousRevisions = dpContractVersions.slice(0, 2)
      .map(({ version, reference }) => (
        new Revision(version, reference)
      ));

    const svContract = new SVContract(
      contractId,
      dpContract,
      dpContractVersions[dpContractVersions.length - 1].reference,
      false,
      previousRevisions,
    );

    await svContractMongoDbRepository.store(svContract);

    // 3. Revert 3rd version of contract to 2nd
    const thirdDPContractVersion = dpContractVersions[dpContractVersions.length - 1];

    await revertSVContractsForStateTransition({
      stateTransition: thirdDPContractVersion.stateTransition,
      block: thirdDPContractVersion.block,
    });

    const revertedSVContract = await svContractMongoDbRepository.find(contractId);

    expect(revertedSVContract.getDPContract().getVersion()).to.be.equal(2);

    expect(revertedSVContract.getPreviousRevisions()).to.be.deep.equal([
      previousRevisions[0],
    ]);

    expect(readerMediator.emitSerial.getCall(1)).to.be.calledWith(
      ReaderMediator.EVENTS.DP_CONTRACT_REVERTED,
      {
        userId: thirdDPContractVersion.stateTransition.extraPayload.regTxId,
        contractId,
        reference: thirdDPContractVersion.reference,
        contract: dpContract.toJSON(),
        previousRevision: previousRevisions[previousRevisions.length - 1],
      },
    );
  });

  it('should delete SV Contract if there are no previous versions', async () => {
    const svContract = getSVContractFixture();
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlocksFixture();
    const reference = getReferenceFixture();

    const contractId = svContract.getContractId();

    await svContractMongoDbRepository.store(svContract);

    await revertSVContractsForStateTransition({
      stateTransition,
      block,
    });

    const revertedSVContract = await svContractMongoDbRepository.find(contractId);

    expect(revertedSVContract).to.not.exist();

    expect(readerMediator.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.DP_CONTRACT_MARKED_DELETED,
      {
        userId: stateTransition.extraPayload.regTxId,
        contractId,
        reference,
        contract: svContract.getDPContract().toJSON(),
      },
    );
  });
});
