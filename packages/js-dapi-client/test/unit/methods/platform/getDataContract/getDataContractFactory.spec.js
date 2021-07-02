const {
  v0: {
    PlatformPromiseClient,
    GetDataContractRequest,
    GetDataContractResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getDataContractFactory = require('../../../../../lib/methods/platform/getDataContract/getDataContractFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const NotFoundError = require('../../../../../lib/methods/errors/NotFoundError');

describe('getDataContractFactory', () => {
  let grpcTransportMock;
  let getDataContract;
  let options;
  let response;
  let dataContractFixture;
  let metadataFixture;

  beforeEach(function beforeEach() {
    dataContractFixture = getDataContractFixture();

    response = new GetDataContractResponse();
    response.setDataContract(dataContractFixture.toBuffer());

    metadataFixture = getMetadataFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response.setMetadata(metadata);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getDataContract = getDataContractFactory(grpcTransportMock);
  });

  it('should return data contract', async () => {
    const contractId = dataContractFixture.getId();
    const result = await getDataContract(contractId, options);

    const request = new GetDataContractRequest();
    request.setId(contractId);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    ]);
    expect(result.getDataContract()).to.deep.equal(dataContractFixture.toBuffer());
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should throw NotFoundError if data contract not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const contractId = dataContractFixture.getId();

    try {
      await getDataContract(contractId, options);

      expect.fail('should throw NotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundError);
    }

    const request = new GetDataContractRequest();
    request.setId(contractId);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    ]);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');
    const contractId = dataContractFixture.getId();

    grpcTransportMock.request.throws(error);

    const request = new GetDataContractRequest();
    request.setId(contractId.toBuffer());

    try {
      await getDataContract(contractId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getDataContract',
        request,
        options,
      );
    }
  });
});
