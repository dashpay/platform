const {
  v0: {
    PlatformPromiseClient,
    GetConsensusParamsResponse,
    GetConsensusParamsRequest,
    ConsensusParamsBlock,
    ConsensusParamsEvidence,
  },
} = require('@dashevo/dapi-grpc');
const getConsensusParamsFactory = require('../../../../../lib/methods/platform/getConsensusParams/getConsensusParamsFactory');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('getConsensusParams', () => {
  let getConsensusParams;
  let grpcTransportMock;
  let response;
  let consensusParamsFixture;

  beforeEach(function beforeEach() {
    consensusParamsFixture = {
      block: {
        timeIotaMs: 1000,
        maxGas: -1,
        maxBytes: 22020103,
      },
      evidence: {
        maxAgeNumBlocks: 100007,
        maxAgeDuration: 172807000000007,
        maxBytes: 1048583,
      },
    };

    const block = new ConsensusParamsBlock();
    block.setMaxBytes(consensusParamsFixture.block.maxBytes);
    block.setMaxGas(consensusParamsFixture.block.maxGas);
    block.setTimeIotaMs(consensusParamsFixture.block.timeIotaMs);

    const evidence = new ConsensusParamsEvidence();
    evidence.setMaxAgeNumBlocks(consensusParamsFixture.evidence.maxAgeNumBlocks);
    evidence.setMaxAgeDuration(consensusParamsFixture.evidence.maxAgeDuration);
    evidence.setMaxBytes(consensusParamsFixture.evidence.maxBytes);

    response = new GetConsensusParamsResponse();
    response.setBlock(block);
    response.setEvidence(evidence);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getConsensusParams = getConsensusParamsFactory(grpcTransportMock);
  });

  it('should return consensus params', async () => {
    const result = await getConsensusParams();
    const options = {};

    const request = new GetConsensusParamsRequest();
    request.setProve(!!options.prove);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getConsensusParams',
      request,
      options,
    ]);

    expect(result.getBlock()).to.deep.equal(consensusParamsFixture.block);
    expect(result.getEvidence()).to.deep.equal(consensusParamsFixture.evidence);
  });

  it('should return consensus params for height', async () => {
    const height = 42;
    const result = await getConsensusParams(height);

    const options = { };

    const request = new GetConsensusParamsRequest();
    request.setProve(!!options.prove);
    request.setHeight(height);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getConsensusParams',
      request,
      options,
    ]);

    expect(result.getBlock()).to.deep.equal(consensusParamsFixture.block);
    expect(result.getEvidence()).to.deep.equal(consensusParamsFixture.evidence);
  });

  it('should return consensus params with proofs', async () => {
    const options = { prove: true };

    const result = await getConsensusParams(undefined, options);

    const request = new GetConsensusParamsRequest();
    request.setProve(!!options.prove);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getConsensusParams',
      request,
      options,
    ]);

    expect(result.getBlock()).to.deep.equal(consensusParamsFixture.block);
    expect(result.getEvidence()).to.deep.equal(consensusParamsFixture.evidence);
  });

  it('should throw InvalidResponseError', async () => {
    const options = {};
    const error = new InvalidResponseError('Unknown error');

    grpcTransportMock.request.throws(error);

    const request = new GetConsensusParamsRequest();
    request.setProve(!!options.prove);

    try {
      await getConsensusParams();

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledThrice();
    }
  });

  it('should throw unknown error', async () => {
    const options = {};
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetConsensusParamsRequest();
    request.setProve(!!options.prove);

    try {
      await getConsensusParams();

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getConsensusParams',
        request,
        options,
      );
    }
  });
});
