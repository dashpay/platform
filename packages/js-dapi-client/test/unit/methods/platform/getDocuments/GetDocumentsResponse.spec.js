const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const {
  v0: {
    GetDocumentsResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const GetDocumentsResponseClass = require('../../../../../lib/methods/platform/getDocuments/GetDocumentsResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetDocumentsResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let documentsFixture;
  let proto;
  let serializedDocuments;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    documentsFixture = getDocumentsFixture();
    proto = new GetDocumentsResponse();

    serializedDocuments = documentsFixture
      .map((document) => Buffer.from(JSON.stringify(document)));

    proto.setDocumentsList(serializedDocuments);

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = new GetDocumentsResponseClass(
      serializedDocuments,
      metadataFixture,
    );
  });

  it('should return documents', () => {
    const documents = getDataContractResponse.getDocuments();

    expect(documents).to.deep.equal(serializedDocuments);
  });

  it('should create an instance from proto', () => {
    getDataContractResponse = GetDocumentsResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDocumentsResponseClass);
    expect(getDataContractResponse.getDocuments()).to.deep.equal(serializedDocuments);
    expect(getDataContractResponse.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getDataContractResponse = GetDocumentsResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
