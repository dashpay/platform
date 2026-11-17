import restartNodeTaskFactory from '../../../../src/listr/tasks/restartNodeTaskFactory.js';

describe('restartNodeTaskFactory', () => {
  it('should title a step so listr renders what stopping prints and why it failed', function it() {
    // listr's default renderer leaves out a subtree whose tasks all lack a
    // title, and with it a DKG wait's progress or a refused stop's reason.
    const restartNodeTask = restartNodeTaskFactory(
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
    );

    const config = { get: this.sinon.stub().returns(null), getName: () => 'mainnet' };

    expect(restartNodeTask(config).tasks.some((task) => task.hasTitle())).to.be.true();
  });
});
