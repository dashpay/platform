const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentitiesByPublicKeyHashesResponseClass = require('../../../../../lib/methods/platform/getIdentitiesByPublicKeyHashes/GetIdentitiesByPublicKeyHashesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetIdentitiesByPublicKeyHashesResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let identityFixture;
  let proto;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();
    proto = new GetIdentitiesByPublicKeyHashesResponse();

    proto.setIdentitiesList(
      [identityFixture.toBuffer()],
    );

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = new GetIdentitiesByPublicKeyHashesResponseClass(
      [identityFixture.toBuffer()],
      metadataFixture,
    );
  });

  it('should return identities', () => {
    const identities = getDataContractResponse.getIdentities();

    expect(identities).to.deep.equal([identityFixture.toBuffer()]);
  });

  it('should create an instance from proto', () => {
    getDataContractResponse = GetIdentitiesByPublicKeyHashesResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(
      GetIdentitiesByPublicKeyHashesResponseClass,
    );
    expect(getDataContractResponse.getIdentities()).to.deep.equal([identityFixture.toBuffer()]);
    expect(getDataContractResponse.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getDataContractResponse = GetIdentitiesByPublicKeyHashesResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
