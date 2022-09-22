const {
  tendermint: {
    abci: {
      ResponseExtendVote,
    },
  },
} = require('@dashevo/abci/types');

const extendVoteHandlerFactory = require('../../../../lib/abci/handlers/extendVoteHandlerFactory');

describe('extendVoteHandlerFactory', () => {
  let extendVoteHandler;

  beforeEach(() => {
    extendVoteHandler = extendVoteHandlerFactory();
  });

  it('should return ResponseExtendVote', async () => {
    const result = await extendVoteHandler();

    expect(result).to.be.an.instanceOf(ResponseExtendVote);
  });
});
