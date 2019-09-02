const {
  mocha: {
    startMongoDb,
  },
} = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const DriveDataProvider = require('../../../lib/dpp/DriveDataProvider');

const Reference = require('../../../lib/stateView/revisions/Reference');

const createSVDocumentMongoDbRepositoryFactory = require('../../../lib/stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const convertWhereToMongoDbQuery = require('../../../lib/stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../../../lib/stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../../../lib/stateView/document/query/findConflictingConditions');
const SVDocumentMongoDbRepository = require('../../../lib/stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const SVContractMongoDbRepository = require('../../../lib/stateView/contract/SVContractMongoDbRepository');

const fetchContractFactory = require('../../../lib/stateView/contract/fetchContractFactory');

const getBlocksFixture = require('../../../lib/test/fixtures/getBlocksFixture');
const getSTPacketsFixture = require('../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');
const getSVContractFixture = require('../../../lib/test/fixtures/getSVContractFixture');

describe.skip('applyStateTransitionFactory', () => {
  let mongoClient;
  let mongoDatabase;
  let svContractMongoDbRepository;
  let createSVDocumentMongoDbRepository;
  let readerMediator;
  let applyStateTransition;

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    svContractMongoDbRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);

    const fetchContract = fetchContractFactory(svContractMongoDbRepository);

    const dataProvider = new DriveDataProvider(
      null,
      fetchContract,
      null,
    );

    dpp.setDataProvider(dataProvider);

    const validateQuery = validateQueryFactory(findConflictingConditions);

    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepository,
      convertWhereToMongoDbQuery,
      validateQuery,
    );
  });

  it('should compute Contract state view', async () => {
    const block = getBlocksFixture()[0];
    const stPacket = getSTPacketsFixture()[0];
    const stateTransition = getStateTransitionsFixture()[0];
    const contractId = stPacket.getContractId();

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    const reference = new Reference({
      blockHash: block.hash,
      blockHeight: block.height,
      stHash: stateTransition.hash,
      stPacketHash: stPacket.hash(),
      hash: stPacket.getContract().hash(),
    });

    await applyStateTransition(stateTransition, block);

    const svContract = await svContractMongoDbRepository.find(contractId);

    expect(svContract.getContractId()).to.equal(contractId);
    expect(svContract.getContract().toJSON()).to.deep.equal(stPacket.getContract().toJSON());
    expect(svContract.getReference()).to.deep.equal(reference);
    expect(svContract.getPreviousRevisions()).to.deep.equal([]);
  });

  it('should compute Documents state view', async () => {
    const svContract = getSVContractFixture();

    svContractMongoDbRepository.store(svContract);

    const block = getBlocksFixture()[1];
    const stPacket = getSTPacketsFixture()[1];
    const stateTransition = getStateTransitionsFixture()[1];

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    await applyStateTransition(stateTransition, block);

    expect(readerMediator.emitSerial).to.have.been.calledTwice();

    for (const document of stPacket.getDocuments()) {
      const svDocumentRepository = createSVDocumentMongoDbRepository(
        stPacket.getContractId(),
        document.getType(),
      );
      const svDocuments = await svDocumentRepository.fetch();

      expect(svDocuments).to.be.an('array');
      expect(svDocuments).to.have.lengthOf(1);

      const [svDocument] = svDocuments;
      const actualDocument = svDocument.getDocument();

      expect(actualDocument.removeMetadata().toJSON()).to.deep.equal(document.toJSON());
    }
  });
});
