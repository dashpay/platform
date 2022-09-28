const {
  v0: {
    GetConsensusParamsResponse,
    ConsensusParamsBlock,
    ConsensusParamsEvidence,
  },
} = require('@dashevo/dapi-grpc');
const {
  server: {
    error: {
      InternalGrpcError,
    },
  },
} = require('@dashevo/grpc-common');
const FailedPreconditionGrpcError = require('@dashevo/grpc-common/lib/server/error/FailedPreconditionGrpcError');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');
const getConsensusParamsHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getConsensusParamsHandlerFactory');
const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const RPCError = require('../../../../../lib/rpcServer/RPCError');

describe('getConsensusParamsHandlerFactory', () => {
  let getConsensusParamsHandler;
  let getConsensusParamsMock;
  let consensusParamsFixture;
  let request;
  let call;

  beforeEach(function beforeEach() {
    request = {
      getHeight: this.sinon.stub().returns(0), // gRPC returns 0 if in parameter is empty
      getProve: this.sinon.stub().returns(false),
    };

    call = new GrpcCallMock(this.sinon, request);

    consensusParamsFixture = {
      block: {
        max_bytes: '22020096',
        max_gas: '1000',
        time_iota_ms: '1000',
      },
      evidence: {
        max_age_num_blocks: '100000',
        max_age_duration: '200000',
        max_bytes: '22020096',
      },
      validator: {
        pub_key_types: [
          'ed25519',
        ],
      },
    };

    getConsensusParamsMock = this.sinon.stub().resolves(consensusParamsFixture);

    getConsensusParamsHandler = getConsensusParamsHandlerFactory(
      getConsensusParamsMock,
    );
  });

  it('should return valid data', async () => {
    const result = await getConsensusParamsHandler(call);

    expect(result).to.be.an.instanceOf(GetConsensusParamsResponse);

    const block = result.getBlock();
    expect(block).to.be.an.instanceOf(ConsensusParamsBlock);
    expect(block.getMaxBytes()).to.equal(consensusParamsFixture.block.max_bytes);
    expect(block.getMaxGas()).to.equal(consensusParamsFixture.block.max_gas);
    expect(block.getTimeIotaMs()).to.equal(consensusParamsFixture.block.time_iota_ms);

    const evidence = result.getEvidence();
    expect(evidence).to.be.an.instanceOf(ConsensusParamsEvidence);
    expect(evidence.getMaxBytes()).to.equal(consensusParamsFixture.evidence.max_bytes);
    expect(evidence.getMaxAgeDuration()).to.equal(consensusParamsFixture.evidence.max_age_duration);
    expect(evidence.getMaxAgeNumBlocks())
      .to.equal(consensusParamsFixture.evidence.max_age_num_blocks);

    expect(getConsensusParamsMock).to.be.calledOnceWith(undefined);
  });

  it('should throw FailedPreconditionGrpcError', async () => {
    const error = new RPCError(32603, 'invalid height', 'some data');
    getConsensusParamsMock.throws(error);

    try {
      await getConsensusParamsHandler(call);

      expect.fail('should throw FailedPreconditionGrpcError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(FailedPreconditionGrpcError);
      expect(e.getMessage()).to.equal('Invalid height: some data');
      expect(e.getCode()).to.equal(9);
    }
  });

  it('should throw InternalGrpcError', async () => {
    const error = new RPCError(32602, 'invalid height', 'some data');
    getConsensusParamsMock.throws(error);

    try {
      await getConsensusParamsHandler(call);

      expect.fail('should throw InternalGrpcError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getError()).to.equal(e.getError());
    }
  });

  it('should throw InvalidArgumentGrpcError', async () => {
    request.getProve.returns(true);

    try {
      await getConsensusParamsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Prove is not implemented yet');
    }
  });

  it('should throw unknown error', async () => {
    const error = new Error('unknown error');
    getConsensusParamsMock.throws(error);

    try {
      await getConsensusParamsHandler(call);

      expect.fail('should throw InternalGrpcError');
    } catch (e) {
      expect(e).to.equal(e);
    }
  });

  it('should return valid data for height', async () => {
    request.getHeight.returns(42);

    const result = await getConsensusParamsHandler(call);

    expect(result).to.be.an.instanceOf(GetConsensusParamsResponse);

    const block = result.getBlock();
    expect(block).to.be.an.instanceOf(ConsensusParamsBlock);
    expect(block.getMaxBytes()).to.equal(consensusParamsFixture.block.max_bytes);
    expect(block.getMaxGas()).to.equal(consensusParamsFixture.block.max_gas);
    expect(block.getTimeIotaMs()).to.equal(consensusParamsFixture.block.time_iota_ms);

    const evidence = result.getEvidence();
    expect(evidence).to.be.an.instanceOf(ConsensusParamsEvidence);
    expect(evidence.getMaxBytes()).to.equal(consensusParamsFixture.evidence.max_bytes);
    expect(evidence.getMaxAgeDuration()).to.equal(consensusParamsFixture.evidence.max_age_duration);
    expect(evidence.getMaxAgeNumBlocks())
      .to.equal(consensusParamsFixture.evidence.max_age_num_blocks);

    expect(getConsensusParamsMock).to.be.calledOnceWith(42);
  });
});
