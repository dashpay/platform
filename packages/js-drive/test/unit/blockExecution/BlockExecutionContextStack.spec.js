const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');
const BlockExecutionContextStack = require('../../../lib/blockExecution/BlockExecutionContextStack');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');
const ContextsAreMoreThanStackMaxSizeError = require('../../../lib/blockExecution/errors/ContextsAreMoreThanStackMaxSizeError');

describe('BlockExecutionContextStack', () => {
  let blockExecutionContextStack;
  let blockExecutionContext;

  beforeEach(() => {
    blockExecutionContextStack = new BlockExecutionContextStack();

    blockExecutionContext = new BlockExecutionContext();
    blockExecutionContext.fromObject(
      getBlockExecutionContextObjectFixture(),
    );
  });

  describe('#setContexts and #getContexts', () => {
    it('should set contexts', () => {
      blockExecutionContextStack.setContexts([
        blockExecutionContext,
      ]);

      expect(blockExecutionContextStack.getContexts()).to.have.members([
        blockExecutionContext,
      ]);
    });

    it('should throw ContextsAreMoreThanStackMaxSizeError error if contexts are more than stack max size', () => {
      try {
        blockExecutionContextStack.setContexts([
          blockExecutionContext,
          blockExecutionContext,
          blockExecutionContext,
          blockExecutionContext,
        ]);

        expect.fail('should throw ContextsAreMoreThanStackMaxSizeError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(ContextsAreMoreThanStackMaxSizeError);
      }
    });
  });

  describe('#getFirst', () => {
    it('should return the first context from the stack', () => {
      let result = blockExecutionContextStack.getFirst();

      expect(result).to.be.undefined();

      blockExecutionContextStack.setContexts([
        blockExecutionContext,
      ]);

      result = blockExecutionContextStack.getFirst();

      expect(result).to.equals(blockExecutionContext);
    });
  });

  describe('#getLast', () => {
    it('should return the last context from the stack', () => {
      let result = blockExecutionContextStack.getLast();

      expect(result).to.be.undefined();

      const lastContext = new BlockExecutionContext();

      blockExecutionContextStack.setContexts([
        blockExecutionContext,
        blockExecutionContext,
        lastContext,
      ]);

      result = blockExecutionContextStack.getLast();

      expect(result).to.equals(lastContext);
    });
  });

  describe('#removeLatest', () => {
    it('should return remove the last context from the stack', () => {
      const lastContext = new BlockExecutionContext();

      blockExecutionContextStack.setContexts([
        blockExecutionContext,
        lastContext,
      ]);

      blockExecutionContextStack.removeLatest();

      const result = blockExecutionContextStack.getContexts();

      expect(result).to.deep.equals([
        blockExecutionContext,
      ]);
    });
  });

  describe('add', () => {
    it('should append a context to the stack and remove the last one', () => {
      const firstContext = new BlockExecutionContext();
      const secondContext = new BlockExecutionContext();
      const thirdContext = new BlockExecutionContext();
      const forthContext = new BlockExecutionContext();

      blockExecutionContextStack.add(firstContext);

      expect(blockExecutionContextStack.getContexts()).to.have.ordered.members([
        firstContext,
      ]);

      blockExecutionContextStack.add(secondContext);

      expect(blockExecutionContextStack.getContexts()).to.have.ordered.members([
        secondContext, firstContext,
      ]);

      blockExecutionContextStack.add(thirdContext);

      expect(blockExecutionContextStack.getContexts()).to.have.ordered.members([
        thirdContext, secondContext, firstContext,
      ]);

      blockExecutionContextStack.add(forthContext);

      expect(blockExecutionContextStack.getContexts()).to.have.ordered.members([
        forthContext, thirdContext, secondContext,
      ]);
    });
  });

  describe('getSize', () => {
    it('should return the current size of the stack', () => {
      blockExecutionContextStack.setContexts([
        blockExecutionContext,
        blockExecutionContext,
      ]);

      const result = blockExecutionContextStack.getSize();
      expect(result).to.equals(2);
    });
  });
});
