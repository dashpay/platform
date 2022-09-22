const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
  },
} = require('@dashevo/abci/types');
const processProposalHandlerFactory = require('../../../../lib/abci/handlers/processProposalHandlerFactory');

describe('processProposalHandlerFactory', () => {
  let processProposalHandler;

  beforeEach(() => {
    processProposalHandler = processProposalHandlerFactory();
  });

  it('should return ResponseProcessProposal', async () => {
    const result = await processProposalHandler();

    expect(result).to.be.an.instanceOf(ResponseProcessProposal);
    expect(result.status).to.equal(1);
  });
});
