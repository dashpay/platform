const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentityResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityResponseClass = require('../../../../../lib/methods/platform/getIdentity/GetIdentityResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetIdentityResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let identityFixture;
  let proto;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();

    proto = new GetIdentityResponse();
    proto.setIdentity(identityFixture.toBuffer());

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = new GetIdentityResponseClass(
      identityFixture.toBuffer(),
      metadataFixture,
    );
  });

  it('should return Identity', () => {
    const identity = getDataContractResponse.getIdentity();

    expect(identity).to.deep.equal(identityFixture.toBuffer());
  });

  it('should create an instance from proto', () => {
    getDataContractResponse = GetIdentityResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetIdentityResponseClass);
    expect(getDataContractResponse.getIdentity()).to.deep.equal(identityFixture.toBuffer());
    expect(getDataContractResponse.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getDataContractResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if Identity is not defined', () => {
    proto.setIdentity(undefined);

    try {
      getDataContractResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
