const {
  tendermint: {
    abci: {
      ResponseExtendVote,
    },
  },
} = require('@dashevo/abci/types');

const { hash } = require('@dashevo/dpp/lib/util/hash');

const extendVoteHandlerFactory = require('../../../../lib/abci/handlers/extendVoteHandlerFactory');

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('extendVoteHandlerFactory', () => {
  let extendVoteHandler;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getWithdrawalTransactionsMap.returns({});

    extendVoteHandler = extendVoteHandlerFactory(blockExecutionContextMock);
  });

  it('should return ResponseExtendVote', async () => {
    const result = await extendVoteHandler();

    expect(result).to.be.an.instanceOf(ResponseExtendVote);
  });

  it('should return ResponseExtendVote with vote extensions if withdrawal transactions are present', async () => {
    const [txOneBytes, txTwoBytes] = [
      Buffer.alloc(32, 0),
      Buffer.alloc(32, 1),
    ];

    blockExecutionContextMock.getWithdrawalTransactionsMap.returns({
      [hash(txOneBytes).toString('hex')]: txOneBytes,
      [hash(txTwoBytes).toString('hex')]: txTwoBytes,
    });

    const result = await extendVoteHandler();

    expect(result).to.be.an.instanceOf(ResponseExtendVote);
    expect(result.voteExtensions).to.deep.equal([
      {
        type: 1,
        extension: hash(txOneBytes),
      },
      {
        type: 1,
        extension: hash(txTwoBytes),
      },
    ]);
  });
});
