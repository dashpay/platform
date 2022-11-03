const ProposalBlockExecutionContextCollection = require('../../../lib/blockExecution/ProposalBlockExecutionContextCollection');
const BlockExecutionContextNotFoundError = require('../../../lib/blockExecution/errors/BlockExecutionContextNotFoundError');
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

  describe('#add', () => {
    it('should add block execution context for the round', () => {
      const result = proposalBlockExecutionContextCollection.add(round, blockExecutionContextMock);

      expect(result).to.be.instanceOf(ProposalBlockExecutionContextCollection);
      expect(proposalBlockExecutionContextCollection.collection.get(round)).to.equal(
        blockExecutionContextMock,
      );
    });
  });

  describe('#set', () => {
    it('should get block execution context for the round', () => {
      proposalBlockExecutionContextCollection.collection.set(round, blockExecutionContextMock);

      const result = proposalBlockExecutionContextCollection.get(round);

      expect(result).to.equal(blockExecutionContextMock);
    });

    it('should throw BlockExecutionContextNotFoundError', () => {
      try {
        proposalBlockExecutionContextCollection.get(-1);

        expect.fail('should throw BlockExecutionContextNotFoundError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionContextNotFoundError);
      }
    });
  });

  describe('#isEmpty', () => {
    it('should return true if collection is empty', () => {
      proposalBlockExecutionContextCollection.collection.clear();

      const result = proposalBlockExecutionContextCollection.isEmpty();

      expect(result).to.be.true();
    });

    it('should return false if collection is not empty', () => {
      proposalBlockExecutionContextCollection.collection.set(round, blockExecutionContextMock);

      const result = proposalBlockExecutionContextCollection.isEmpty();

      expect(result).to.be.false();
    });
  });
});
