const { expect } = require('chai');

const createTestDIContainer = require('../../lib/test/createTestDIContainer');

describe('createDIContainer', function describeContainer() {
  this.timeout(25000);

  let container;

  beforeEach(async () => {
    container = await createTestDIContainer();
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should create DI container', async () => {
    expect(container).to.respondTo('register');
    expect(container).to.respondTo('resolve');
  });

  describe('container', () => {
    it('should resolve abciHandlers', () => {
      const abciHandlers = container.resolve('abciHandlers');

      expect(abciHandlers).to.have.property('info');
      expect(abciHandlers).to.have.property('checkTx');
      expect(abciHandlers).to.have.property('finalizeBlock');
      expect(abciHandlers).to.have.property('extendVote');
      expect(abciHandlers).to.have.property('initChain');
      expect(abciHandlers).to.have.property('prepareProposal');
      expect(abciHandlers).to.have.property('processProposal');
      expect(abciHandlers).to.have.property('verifyVoteExtension');
      expect(abciHandlers).to.have.property('query');
    });
  });
});
