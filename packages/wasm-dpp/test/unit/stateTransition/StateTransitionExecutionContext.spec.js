const { expect } = require('chai');
const { default: loadWasmDpp } = require('../../../dist');

describe.skip('StateTransitionExecutionContext', () => {
  let ReadOperation;
  let PreCalculatedOperation;
  let SignatureVerificationOperation;
  let StateTransitionExecutionContext;

  let executionContext;

  before(async () => {
    ({
      ReadOperation,
      PreCalculatedOperation,
      SignatureVerificationOperation,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    executionContext = new StateTransitionExecutionContext();
  });

  describe('should add operation', () => {
    it('should add ReadOperation', () => {
      executionContext.addOperation(new ReadOperation(100));
      const operations = executionContext.getOperations();
      expect(operations[0]).to.be.instanceOf(ReadOperation);
    });

    it('should add SignatureVerificationOperation', () => {
      executionContext.addOperation(new SignatureVerificationOperation(0));
      const operations = executionContext.getOperations();
      expect(operations[0]).to.be.instanceOf(SignatureVerificationOperation);
    });

    it('should add PreCalculatedOperation', () => {
      executionContext.addOperation(new PreCalculatedOperation(1, 1, [{
        identifier: Buffer.alloc(32).fill(32),
        creditsPerEpoch: { 0: 9991498 },
      }]));
      const operations = executionContext.getOperations();
      expect(operations[0]).to.be.instanceOf(PreCalculatedOperation);
    });

    it('should throw Error for unknown operation', () => {
      try {
        executionContext.addOperation({});
      } catch (e) {
        expect(e.message).to.equal('Unknown operation');
      }
    });
  });
});
