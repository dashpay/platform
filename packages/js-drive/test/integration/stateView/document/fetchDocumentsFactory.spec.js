const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const SVDocumentMongoDbRepository = require('../../../../lib/stateView/document/SVDocumentMongoDbRepository');

const sanitizer = require('../../../../lib/mongoDb/sanitizer');
const createSVDocumentMongoDbRepositoryFactory = require('../../../../lib/stateView/document/createSVDocumentMongoDbRepositoryFactory');
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

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
  });

  beforeEach(() => {
    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepository,
      sanitizer,
    );

    fetchDocuments = fetchDocumentsFactory(createSVDocumentMongoDbRepository);

    [svDocument] = getSVDocumentsFixture();

    document = svDocument.getDocument();
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

    expect(actualDocument.toJSON()).to.deep.equal(document.toJSON());
  });

  it('should fetch Documents for specified contract id, document type and name', async () => {
    let result = await fetchDocuments(contractId, type);

    expect(result).to.deep.equal([]);

    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);
    await svDocumentRepository.store(svDocument);

    const options = { where: { 'document.name': document.get('name') } };
    result = await fetchDocuments(contractId, type, options);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    expect(actualDocument.toJSON()).to.deep.equal(document.toJSON());
  });

  it('should return empty array for specified contract ID, document type and name not exist', async () => {
    const svDocumentRepository = createSVDocumentMongoDbRepository(contractId, type);
    await svDocumentRepository.store(svDocument);

    const options = { where: { 'document.name': 'unknown' } };

    const result = await fetchDocuments(contractId, type, options);

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
