const {
  tendermint: {
    abci: {
      ResponseVerifyVoteExtension,
    },
  },
} = require('@dashevo/abci/types');
const verifyVoteExtensionHandlerFactory = require('../../../../lib/abci/handlers/verifyVoteExtensionHandlerFactory');

describe('verifyVoteExtensionHandlerFactory', () => {
  let verifyVoteExtensionHandler;

  beforeEach(() => {
    verifyVoteExtensionHandler = verifyVoteExtensionHandlerFactory();
  });

  it('should return ResponseVerifyVoteExtension', async () => {
    const result = await verifyVoteExtensionHandler();

    expect(result).to.be.an.instanceOf(ResponseVerifyVoteExtension);
    expect(result.status).to.equal(1);
  });
});
