const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');

const getIdentitiesByPublicKeyHashesFactory = require(
  '../../../../../lib/methods/platform/getIdentitiesByPublicKeyHashes/getIdentitiesByPublicKeyHashesFactory',
);

describe('getIdentitiesByPublicKeyHashesFactory', () => {
  let grpcTransportMock;
  let getIdentitiesByPublicKeyHashes;
  let options;
  let response;
  let identityFixture;
  let publicKeyHash;
  let metadataFixture;

  beforeEach(function beforeEach() {
    identityFixture = getIdentityFixture();
    metadataFixture = getMetadataFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setIdentitiesList(
      [identityFixture.toBuffer()],
    );
    response.setMetadata(metadata);

    publicKeyHash = identityFixture.getPublicKeyById(1).hash();

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getIdentitiesByPublicKeyHashes = getIdentitiesByPublicKeyHashesFactory(grpcTransportMock);
  });

  it('should return public key hashes to identity map', async () => {
    const result = await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setPublicKeyHashesList([publicKeyHash]);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      request,
      options,
    );
    expect(result.getIdentities()).to.have.deep.members([identityFixture.toBuffer()]);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setPublicKeyHashesList([publicKeyHash]);

    try {
      await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentitiesByPublicKeyHashes',
        request,
        options,
      );
    }
  });
});
