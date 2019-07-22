const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const SVDocumentMongoDbRepository = require('../../../../lib/stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const convertWhereToMongoDbQuery = require('../../../../lib/stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../../../../lib/stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../../../../lib/stateView/document/query/findConflictingConditions');

const createSVDocumentMongoDbRepositoryFactory = require('../../../../lib/stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const fetchDocumentsFactory = require('../../../../lib/stateView/document/fetchDocumentsFactory');

const getSVDocumentsFixture = require('../../../../lib/test/fixtures/getSVDocumentsFixture');

describe('fetchDocumentsFactory', () => {
  let createSVDocumentMongoDbRepository;
  let fetchDocuments;
  let mongoClient;
  let svDocument;
  let type;
  let contractId;
  let document;
  let stReference;

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
  });

  beforeEach(() => {
    const validateQuery = validateQueryFactory(findConflictingConditions);

    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepository,
      convertWhereToMongoDbQuery,
      validateQuery,
    );

    fetchDocuments = fetchDocumentsFactory(createSVDocumentMongoDbRepository);

    [svDocument] = getSVDocumentsFixture();

    const currentReference = svDocument.getCurrentRevision()
      .getReference();

    document = svDocument.getDocument();
    stReference = {
      blockHash: currentReference.getBlockHash(),
      blockHeight: currentReference.getBlockHeight(),
      stHeaderHash: currentReference.getSTHash(),
      stPacketHash: currentReference.getSTPacketHash(),
    };
    type = document.getType();
    contractId = 'HgKXrLhm7sMjPrRGS1UsETmmQ7nZHbaKN729zw55PUVk';
  });

  it('should fetch Documents for specified contract ID and document type', async () => {
    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);
    await svDocumentRepository.store(svDocument);

    const result = await fetchDocuments(contractId, type);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    const documentJSON = document.toJSON();

    documentJSON.$meta.stReference = stReference;

    expect(actualDocument.toJSON()).to.deep.equal(documentJSON);
  });

  it('should fetch Documents for specified contract id, document type and name', async () => {
    let result = await fetchDocuments(contractId, type);

    expect(result).to.deep.equal([]);

    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);
    await svDocumentRepository.store(svDocument);

    const query = { where: [['name', '==', document.get('name')]] };
    result = await fetchDocuments(contractId, type, query);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    const documentJSON = document.toJSON();

    documentJSON.$meta.stReference = stReference;

    expect(actualDocument.toJSON()).to.deep.equal(documentJSON);
  });

  it('should return empty array for specified contract ID, document type and name not exist', async () => {
    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);
    await svDocumentRepository.store(svDocument);

    const query = { where: [['name', '==', 'unknown']] };

    const result = await fetchDocuments(contractId, type, query);

    expect(result).to.deep.equal([]);
  });

  it('should return empty array if contract ID does not exist', async () => {
    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);

    await svDocumentRepository.store(svDocument);

    contractId = 'Unknown';

    const result = await fetchDocuments(contractId, type);

    expect(result).to.deep.equal([]);
  });

  it('should return empty array if type does not exist', async () => {
    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);

    await svDocumentRepository.store(svDocument);

    type = 'Unknown';

    const result = await fetchDocuments(contractId, type);

    expect(result).to.deep.equal([]);
  });
});
