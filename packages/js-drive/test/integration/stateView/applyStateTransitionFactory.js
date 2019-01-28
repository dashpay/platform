const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const DriveDataProvider = require('../../../lib/dpp/DriveDataProvider');

const Reference = require('../../../lib/stateView/revisions/Reference');

const sanitizer = require('../../../lib/mongoDb/sanitizer');

const createSVObjectMongoDbRepositoryFactory = require('../../../lib/stateView/object/createSVObjectMongoDbRepositoryFactory');
const SVObjectMongoDbRepository = require('../../../lib/stateView/object/SVObjectMongoDbRepository');
const SVContractMongoDbRepository = require('../../../lib/stateView/contract/SVContractMongoDbRepository');
const updateSVContractFactory = require('../../../lib/stateView/contract/updateSVContractFactory');
const updateSVObjectFactory = require('../../../lib/stateView/object/updateSVObjectFactory');
const applyStateTransitionFactory = require('../../../lib/stateView/applyStateTransitionFactory');

const fetchDPContractFactory = require('../../../lib/stateView/contract/fetchDPContractFactory');
const addSTPacketFactory = require('../../../lib/storage/stPacket/addSTPacketFactory');
const STPacketIpfsRepository = require('../../../lib/storage/stPacket/STPacketIpfsRepository');

const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');
const ReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const getBlocksFixture = require('../../../lib/test/fixtures/getBlocksFixture');
const getSTPacketsFixture = require('../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');
const getSVContractFixture = require('../../../lib/test/fixtures/getSVContractFixture');

describe('applyStateTransitionFactory', () => {
  let mongoClient;
  let mongoDb;
  let ipfsClient;
  let addSTPacket;
  let svContractMongoDbRepository;
  let createSVObjectMongoDbRepository;
  let readerMediator;
  let applyStateTransition;

  startMongoDb().then((mongoDbInstance) => {
    mongoClient = mongoDbInstance.getClient();
    mongoDb = mongoDbInstance.getDb();
  });

  startIPFS().then((ipfsInstance) => {
    ipfsClient = ipfsInstance.getApi();
  });

  beforeEach(function beforeEach() {
    const dpp = new DashPlatformProtocol();

    svContractMongoDbRepository = new SVContractMongoDbRepository(mongoDb, dpp);

    const createFetchDPContract = () => fetchDPContractFactory(svContractMongoDbRepository);

    const dataProvider = new DriveDataProvider(
      null,
      createFetchDPContract,
      null,
    );

    dpp.setDataProvider(dataProvider);

    const stPacketRepository = new STPacketIpfsRepository(
      ipfsClient,
      dpp,
      1000,
    );

    addSTPacket = addSTPacketFactory(stPacketRepository);

    createSVObjectMongoDbRepository = createSVObjectMongoDbRepositoryFactory(
      mongoClient,
      SVObjectMongoDbRepository,
      sanitizer,
    );

    const updateSVContract = updateSVContractFactory(svContractMongoDbRepository);
    const updateSVObject = updateSVObjectFactory(createSVObjectMongoDbRepository);
    readerMediator = new ReaderMediatorMock(this.sinon);
    applyStateTransition = applyStateTransitionFactory(
      stPacketRepository,
      updateSVContract,
      updateSVObject,
      readerMediator,
    );
  });

  it('should compute DP Contract state view', async () => {
    const block = getBlocksFixture()[0];
    const stPacket = getSTPacketsFixture()[0];
    const stateTransition = getStateTransitionsFixture()[0];
    const contractId = stPacket.getDPContractId();

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    const reference = new Reference({
      blockHash: block.hash,
      blockHeight: block.height,
      stHash: stateTransition.hash,
      stPacketHash: stPacket.hash(),
      hash: stPacket.getDPContract().hash(),
    });

    await addSTPacket(stPacket);

    await applyStateTransition(stateTransition, block);

    expect(readerMediator.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.DP_CONTRACT_APPLIED,
      {
        userId: stateTransition.extraPayload.regTxId,
        contractId,
        reference,
        contract: stPacket.getDPContract().toJSON(),
      },
    );

    const svContract = await svContractMongoDbRepository.find(contractId);

    expect(svContract.getContractId()).to.be.equal(contractId);
    expect(svContract.getDPContract().toJSON()).to.be.deep.equal(stPacket.getDPContract().toJSON());
    expect(svContract.getReference()).to.be.deep.equal(reference);
    expect(svContract.getPreviousRevisions()).to.be.deep.equal([]);
  });

  it('should compute DP Objects state view', async () => {
    const svContract = getSVContractFixture();

    svContractMongoDbRepository.store(svContract);

    const block = getBlocksFixture()[1];
    const stPacket = getSTPacketsFixture()[1];
    const stateTransition = getStateTransitionsFixture()[1];

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    await addSTPacket(stPacket);

    await applyStateTransition(stateTransition, block);

    expect(readerMediator.emitSerial).to.be.calledTwice();

    for (const dpObject of stPacket.getDPObjects()) {
      const svObjectRepository = createSVObjectMongoDbRepository(
        stPacket.getDPContractId(),
        dpObject.getType(),
      );
      const svObjects = await svObjectRepository.fetch();

      expect(svObjects).to.be.a('array');
      expect(svObjects).to.have.lengthOf(1);

      const [svObject] = svObjects;

      expect(svObject.getDPObject().toJSON()).to.be.deep.equal(dpObject.toJSON());

      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stPacket.hash(),
        hash: dpObject.hash(),
      });

      expect(readerMediator.emitSerial).to.be.calledWith(
        ReaderMediator.EVENTS.DP_OBJECT_APPLIED,
        {
          userId: stateTransition.extraPayload.regTxId,
          contractId: stPacket.getDPContractId(),
          objectId: dpObject.getId(),
          reference,
          object: dpObject.toJSON(),
        },
      );
    }
  });
});
