const ProposalBlockExecutionContextCollection = require('../../../lib/blockExecution/ProposalBlockExecutionContextCollection');
const BlockExecutionContextMock = require('../../../lib/test/mock/BlockExecutionContextMock');

describe('ProposalBlockExecutionContextCollection', () => {
  let proposalBlockExecutionContextCollection;
  let blockExecutionContextMock;
  let round;

  beforeEach(function beforeEach() {
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    proposalBlockExecutionContextCollection = new ProposalBlockExecutionContextCollection();
    round = 42;
  });

  it('should add block execution context for the round', () => {
    const result = proposalBlockExecutionContextCollection.add(round, blockExecutionContextMock);

    expect(result).to.be.instanceOf(ProposalBlockExecutionContextCollection);
    expect(proposalBlockExecutionContextCollection.collection.get(round)).to.equal(
      blockExecutionContextMock,
    );
  });

  it('should get block execution context for the round', () => {
    proposalBlockExecutionContextCollection.collection.set(round, blockExecutionContextMock);

    const result = proposalBlockExecutionContextCollection.get(round);

    expect(result).to.equal(blockExecutionContextMock);
  });
});
