const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityIdsByPublicKeyHashesResponseClass = require('../../../../../lib/methods/platform/getIdentityIdsByPublicKeyHashes/GetIdentityIdsByPublicKeyHashesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetIdentityIdsByPublicKeyHashesResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let identityFixture;
  let proto;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();

    proto = new GetIdentityIdsByPublicKeyHashesResponse();
    proto.setIdentityIdsList(
      [identityFixture.getId()],
    );

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = new GetIdentityIdsByPublicKeyHashesResponseClass(
      [identityFixture.getId()],
      metadataFixture,
    );
  });

  it('should return Identity IDs', () => {
    const identityIds = getDataContractResponse.getIdentityIds();

    expect(identityIds).to.deep.equal([identityFixture.getId()]);
  });

  it('should create an instance from proto', () => {
    getDataContractResponse = GetIdentityIdsByPublicKeyHashesResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(
      GetIdentityIdsByPublicKeyHashesResponseClass,
    );
    expect(getDataContractResponse.getIdentityIds()).to.deep.equal([identityFixture.getId()]);
    expect(getDataContractResponse.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getDataContractResponse = GetIdentityIdsByPublicKeyHashesResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
