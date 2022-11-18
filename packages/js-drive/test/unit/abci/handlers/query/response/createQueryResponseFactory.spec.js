const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');
const {
  google: {
    protobuf: {
      Timestamp,
    },
  },
} = require('@dashevo/abci/types');

const BlockExecutionContextMock = require('../../../../../../lib/test/mock/BlockExecutionContextMock');
const createQueryResponseFactory = require('../../../../../../lib/abci/handlers/query/response/createQueryResponseFactory');
const timeToMillis = require('../../../../../../lib/util/timeToMillis');

describe('createQueryResponseFactory', () => {
  let createQueryResponse;
  let metadata;
  let lastCommitInfo;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    const version = {
      app: 1,
    };

    const time = new Timestamp({
      seconds: Math.ceil(new Date().getTime() / 1000),
      nanos: 0,
    });

    lastCommitInfo = {
      quorumHash: Buffer.alloc(12).fill(1),
      blockSignature: Buffer.alloc(12).fill(2),
    };

    metadata = {
      height: 1,
      coreChainLockedHeight: 1,
      timeMs: timeToMillis(time.seconds, time.nanos),
      protocolVersion: version.app,
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getHeight.returns(metadata.height);
    blockExecutionContextMock.getCoreChainLockedHeight.returns(metadata.coreChainLockedHeight);
    blockExecutionContextMock.getTime.returns(time);
    blockExecutionContextMock.getVersion.returns(version);
    blockExecutionContextMock.getLastCommitInfo.returns(lastCommitInfo);
    blockExecutionContextMock.isEmpty.returns(false);
    blockExecutionContextMock.getRound.returns(42);

    createQueryResponse = createQueryResponseFactory(
      blockExecutionContextMock,
    );
  });

  it('should create a response', () => {
    const response = createQueryResponse(GetDataContractResponse);

    response.serializeBinary();

    expect(response).to.be.instanceOf(GetDataContractResponse);
    expect(response.getMetadata().toObject()).to.deep.equal(metadata);
    expect(response.getProof()).to.undefined();
  });

  it('should create a response with proof if requested', () => {
    const response = createQueryResponse(GetDataContractResponse, true);

    response.serializeBinary();

    expect(response).to.be.instanceOf(GetDataContractResponse);

    expect(response.getMetadata().toObject()).to.deep.equal(metadata);

    expect(response.getProof().toObject()).to.deep.equal({
      quorumHash: lastCommitInfo.quorumHash.toString('base64'),
      signature: lastCommitInfo.blockSignature.toString('base64'),
      merkleProof: '',
      round: 42,
    });
  });
});
