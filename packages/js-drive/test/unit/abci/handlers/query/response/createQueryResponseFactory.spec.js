const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

const BlockExecutionContextMock = require('../../../../../../lib/test/mock/BlockExecutionContextMock');
const createQueryResponseFactory = require('../../../../../../lib/abci/handlers/query/response/createQueryResponseFactory');
const BlockExecutionContextStackMock = require('../../../../../../lib/test/mock/BlockExecutionContextStackMock');

describe('createQueryResponseFactory', () => {
  let blockExecutionContextStackMock;
  let createQueryResponse;
  let metadata;
  let lastCommitInfo;
  let signedBlockExecutionContext;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    signedBlockExecutionContext = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);

    blockExecutionContextStackMock.getLast.returns(signedBlockExecutionContext);
    blockExecutionContextStackMock.getFirst.returns(blockExecutionContextMock);

    metadata = {
      height: 1,
      coreChainLockedHeight: 1,
    };

    signedBlockExecutionContext.getHeader.returns(metadata);

    lastCommitInfo = {
      quorumHash: Buffer.alloc(12).fill(1),
      stateSignature: Buffer.alloc(12).fill(2),
    };

    blockExecutionContextMock.getLastCommitInfo.returns(lastCommitInfo);

    createQueryResponse = createQueryResponseFactory(
      blockExecutionContextStackMock,
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
      signatureLlmqHash: lastCommitInfo.quorumHash.toString('base64'),
      signature: lastCommitInfo.stateSignature.toString('base64'),
      merkleProof: '',
    });
  });
});
