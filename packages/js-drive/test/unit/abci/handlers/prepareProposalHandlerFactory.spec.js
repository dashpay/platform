const {
  tendermint: {
    abci: {
      ResponsePrepareProposal,
    },
  },
} = require('@dashevo/abci/types');
const prepareProposalHandlerFactory = require('../../../../lib/abci/handlers/prepareProposalHandlerFactory');

describe('prepareProposalHandlerFactory', () => {
  let prepareProposalHandler;
  let request;

  beforeEach(() => {
    prepareProposalHandler = prepareProposalHandlerFactory();
    const maxTxBytes = 42;
    const txs = new Array(3).fill(Buffer.alloc(5, 0));

    request = {
      maxTxBytes,
      txs,
    };
  });

  it('should return proposal', async () => {
    const result = await prepareProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponsePrepareProposal);

    expect(result.txRecords).to.deep.equal(
      request.txs.map((tx) => ({
        tx,
        action: 1,
      })),
    );
  });

  it('should cut txs that are not fit into the size limit', async () => {
    request.maxTxBytes = 9;

    const result = await prepareProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponsePrepareProposal);
    expect(result.txRecords).to.have.lengthOf(1);
    expect(result.txRecords).to.deep.equal(
      [{
        tx: request.txs[0],
        action: 1,
      }],
    );
  });
});
