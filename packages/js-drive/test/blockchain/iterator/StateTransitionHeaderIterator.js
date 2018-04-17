const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');
const RpcBlockIterator = require('../../../lib/blockchain/iterator/RpcBlockIterator');
const StateTransitionHeaderIterator = require('../../../lib/blockchain/iterator/StateTransitionHeaderIterator');

describe('StateTransitionHeaderIterator', () => {
  let rpcClientMock;
  let blockIterator;
  let stateTransitionHeaderIterator;

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);
    blockIterator = new RpcBlockIterator(rpcClientMock);
    stateTransitionHeaderIterator = new StateTransitionHeaderIterator(blockIterator, rpcClientMock);

    this.sinon.spy(blockIterator, 'next');
  });

  it('should iterate over ST from blocks from BlockIterator', async () => {
    const obtainedTransitionHeaders = [];

    let done;
    let header;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: header } = await stateTransitionHeaderIterator.next()) {
      if (done) {
        break;
      }

      obtainedTransitionHeaders.push(header);
    }

    expect(rpcClientMock.getTransitionHeader).has.callCount(rpcClientMock.transitionHeaders.length);
    expect(blockIterator.next).has.callCount(rpcClientMock.blocks.length + 1);

    const obtainedHeaderHashes = obtainedTransitionHeaders.map(h => h.getHash());
    const transitionHeaderHashes = rpcClientMock.transitionHeaders.map(h => h.getHash());

    expect(obtainedHeaderHashes).to.be.deep.equal(transitionHeaderHashes);
  });

  it('should iterate from begging when "reset" method is called', async () => {
    const { value: firstHeader } = await stateTransitionHeaderIterator.next();

    stateTransitionHeaderIterator.reset();

    const { value: secondHeader } = await stateTransitionHeaderIterator.next();

    expect(firstHeader.getHash()).to.be.equal(secondHeader.getHash());
  });
});
