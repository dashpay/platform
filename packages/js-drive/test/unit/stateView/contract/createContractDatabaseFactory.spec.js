const createContractDatabaseFactory = require('../../../../lib/stateView/contract/createContractDatabaseFactory');
const SVContract = require('../../../../lib/stateView/contract/SVContract');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('createContractDatabaseFactory', () => {
  let createContractDatabase;
  let svContract;
  let createCollection;
  let convertToMongoDbIndices;
  let convertedIndices;
  let createSVDocumentRepository;

  beforeEach(function beforeEach() {
    const contract = getDataContractFixture();
    const reference = getReferenceFixture();

    const isDeleted = false;
    const previousRevisions = [];

    svContract = new SVContract(
      contract,
      reference,
      isDeleted,
      previousRevisions,
    );

    createCollection = this.sinon.stub();

    const svDocumentRepository = {
      createCollection,
    };

    createSVDocumentRepository = this.sinon.stub().returns(svDocumentRepository);

    convertedIndices = [{
      key: {
        $userId: 1,
        firstName: -1,
      },
      unique: true,
      name: '$userId_firstName',
    }, {
      key: {
        $userId: 1,
        lastName: -1,
      },
      unique: false,
      name: '$userId_lastName',
    }];

    convertToMongoDbIndices = this.sinon.stub().returns(convertedIndices);

    createContractDatabase = createContractDatabaseFactory(
      createSVDocumentRepository,
      convertToMongoDbIndices,
    );
  });

  it('should create all collections with indices', async () => {
    await createContractDatabase(svContract);

    const documents = svContract.getDataContract().getDocuments();
    const { indices: documentIndices } = documents.niceDocument;

    expect(createCollection).to.be.callCount(Object.keys(documents).length);

    expect(convertToMongoDbIndices).to.be.calledOnceWith(documentIndices);
    Object.entries(documents).forEach(([documentType, document]) => {
      expect(createCollection).to.be.calledWith(document.indices ? convertedIndices : undefined);
      expect(createSVDocumentRepository).to.be.calledWith(svContract.getId(), documentType);
    });
  });
});
