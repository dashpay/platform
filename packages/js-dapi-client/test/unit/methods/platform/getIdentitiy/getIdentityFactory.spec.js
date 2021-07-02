const {
  v0: {
    PlatformPromiseClient,
    GetIdentityRequest,
    GetIdentityResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getIdentityFactory = require('../../../../../lib/methods/platform/getIdentity/getIdentityFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const NotFoundError = require('../../../../../lib/methods/errors/NotFoundError');

describe('getIdentityFactory', () => {
  let grpcTransportMock;
  let getIdentity;
  let options;
  let response;
  let identityFixture;
  let identityId;
  let metadataFixture;

  beforeEach(function beforeEach() {
    identityFixture = getIdentityFixture();
    identityId = identityFixture.getId();

    metadataFixture = getMetadataFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response = new GetIdentityResponse();
    response.setIdentity(identityFixture.toBuffer());
    response.setMetadata(metadata);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getIdentity = getIdentityFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return identity', async () => {
    const result = await getIdentity(identityId, options);

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );
    expect(result.getIdentity()).to.deep.equal(identityFixture.toBuffer());
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
  });

  it('should throw NotFoundError if identity not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);
    try {
      await getIdentity(identityId, options);

      expect.fail('should throw NotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundError);
    }

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());

    try {
      await getIdentity(identityId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentity',
        request,
        options,
      );
    }
  });
});
