const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const {
  v0: {
    GetDataContractResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const GetDataContractResponseClass = require('../../../../../lib/methods/platform/getDataContract/GetDataContractResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetDataContractResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let dataContractFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    dataContractFixture = getDataContractFixture();

    getDataContractResponse = new GetDataContractResponseClass(
      dataContractFixture.toBuffer(),
      metadataFixture,
    );
  });

  it('should return DataContract', () => {
    const dataContract = getDataContractResponse.getDataContract();

    expect(dataContract).to.deep.equal(dataContractFixture.toBuffer());
  });

  it('should create an instance from proto', () => {
    const proto = new GetDataContractResponse();
    proto.setDataContract(dataContractFixture.toBuffer());

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDataContractResponseClass);
    expect(getDataContractResponse.getDataContract()).to.deep.equal(dataContractFixture.toBuffer());
    expect(getDataContractResponse.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    const proto = new GetDataContractResponse();
    proto.setDataContract(dataContractFixture.toBuffer());

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if DataContract is not defined', () => {
    const proto = new GetDataContractResponse();
    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
