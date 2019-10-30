const {
  mocha: {
    startMongoDb,
  },
} = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const Reference = require('../../../lib/stateView/revisions/Reference');

const createSVDocumentMongoDbRepositoryFactory = require('../../../lib/stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const convertWhereToMongoDbQuery = require('../../../lib/stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../../../lib/stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../../../lib/stateView/document/query/findConflictingConditions');
const SVDocumentMongoDbRepository = require('../../../lib/stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const SVContractMongoDbRepository = require('../../../lib/stateView/contract/SVContractMongoDbRepository');

const getBlocksFixture = require('../../../lib/test/fixtures/getBlocksFixture');
const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');
const getSVContractFixture = require('../../../lib/test/fixtures/getSVContractFixture');

const updateSVContractFactory = require('../../../lib/stateView/contract/updateSVContractFactory');
const updateSVDocumentFactory = require('../../../lib/stateView/document/updateSVDocumentFactory');

const applyStateTransitionFactory = require('../../../lib/stateView/applyStateTransitionFactory');

describe('applyStateTransitionFactory', () => {
  let mongoClient;
  let mongoDatabase;
  let svContractMongoDbRepository;
  let createSVDocumentMongoDbRepository;
  let applyStateTransition;

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    svContractMongoDbRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);

    const validateQuery = validateQueryFactory(findConflictingConditions);

    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepository,
      convertWhereToMongoDbQuery,
      validateQuery,
    );

    const updateSVContract = updateSVContractFactory(svContractMongoDbRepository);
    const updateSVDocument = updateSVDocumentFactory(createSVDocumentMongoDbRepository);

    applyStateTransition = applyStateTransitionFactory(
      updateSVContract,
      updateSVDocument,
    );
  });

  it('should compute Contract state view', async () => {
    const block = getBlocksFixture()[0];
    const stateTransition = getStateTransitionsFixture()[0];
    const contract = stateTransition.getDataContract();

    const reference = new Reference({
      blockHash: block.hash,
      blockHeight: block.height,
      stHash: stateTransition.hash(),
      hash: contract.hash(),
    });

    await applyStateTransition(stateTransition, block.hash, block.height);

    const svContract = await svContractMongoDbRepository.find(contract.getId());

    expect(svContract.getId()).to.equal(contract.getId());
    expect(svContract.getDataContract().toJSON()).to.deep.equal(contract.toJSON());
    expect(svContract.getReference()).to.deep.equal(reference);
    expect(svContract.getPreviousRevisions()).to.deep.equal([]);
  });

  it('should compute Documents state view', async () => {
    const svContract = getSVContractFixture();

    svContractMongoDbRepository.store(svContract);

    const block = getBlocksFixture()[1];
    const stateTransition = getStateTransitionsFixture()[1];

    await applyStateTransition(stateTransition, block.hash, block.height);

    const [
      documentA,
      documentB,
      documentC,
    ] = stateTransition.getDocuments();

    const documentTypes = ['niceDocument', 'prettyDocument'];
    const documentByTypes = {
      niceDocument: [documentA],
      prettyDocument: [documentB, documentC],
    };

    for (const documentType of documentTypes) {
      const svDocumentRepository = createSVDocumentMongoDbRepository(
        documentA.getDataContractId(),
        documentType,
      );
      const svDocuments = await svDocumentRepository.fetch();

      expect(svDocuments).to.be.an('array');
      expect(svDocuments).to.have.a.lengthOf(
        documentByTypes[documentType].length,
      );

      const actualDocuments = svDocuments.map(svD => svD.getDocument());

      expect(actualDocuments.map(d => d.toJSON())).to.have.deep.members(
        documentByTypes[documentType].map(d => d.toJSON()),
      );
    }
  });
});
