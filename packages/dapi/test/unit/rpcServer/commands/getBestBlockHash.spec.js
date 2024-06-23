const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const getBestBlockHashFactory = require('../../../../lib/rpcServer/commands/getBestBlockHash');

chai.use(chaiAsPromised);

const { expect } = chai;

describe('getBestBlockHash', () => {
  let getBestBlockHash;
  let coreRPCClientMock;
  let zmqClientMock;
  let blockHash;

  beforeEach(function beforeEach() {
    blockHash = '000000000074fc08fb6a92cb8994b14307038261e4266abc6994fa03955a1a59';

    coreRPCClientMock = {
      getBestBlockHash: this.sinon.stub().resolves(blockHash),
    };

    zmqClientMock = { on: this.sinon.stub(), topics: { hashblock: 'fake' } };

    getBestBlockHash = getBestBlockHashFactory(coreRPCClientMock, zmqClientMock);
  });

  it('Should return a number', async () => {
    const bestBlockHash = await getBestBlockHash();
    expect(bestBlockHash).to.equals(blockHash);
    expect(coreRPCClientMock.getBestBlockHash).to.be.calledOnce();
  });
});
