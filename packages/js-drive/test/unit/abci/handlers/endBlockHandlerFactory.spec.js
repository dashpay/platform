const {
  abci: {
    ResponseEndBlock,
  },
} = require('abci/types');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const endBlockHandlerFactory = require('../../../../lib/abci/handlers/endBlockHandlerFactory');
const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionDBTransactionsMock');
const NoDPNSContractFoundError = require('../../../../lib/abci/handlers/errors/NoDPNSContractFoundError');

describe('endBlockHandlerFactory', () => {
  let endBlockHandler;
  let dataContractRepositoryMock;
  let request;
  let blockExecutionDBTransactionsMock;
  let header;
  let dpnsContractId;
  let dpnsContractBlockHeight;
  let blockHeight;
  let transaction;
  let loggerMock;

  beforeEach(function beforeEach() {
    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);

    loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    dataContractRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    transaction = {
      id: 'someTx',
    };

    blockExecutionDBTransactionsMock.getTransaction.returns(transaction);

    endBlockHandler = endBlockHandlerFactory(
      blockExecutionDBTransactionsMock,
      dataContractRepositoryMock,
      dpnsContractBlockHeight,
      dpnsContractId,
      loggerMock,
    );

    blockHeight = 2;

    header = {
      version: {
        App: 1,
      },
      height: blockHeight,
      time: {
        seconds: Math.ceil(new Date().getTime() / 1000),
      },
    };

    request = {
      header,
    };
  });

  it('should simply return a response if DPNS contract was not set', async () => {
    const response = await endBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseEndBlock);

    expect(blockExecutionDBTransactionsMock.start).to.not.have.been.called();
    expect(dataContractRepositoryMock.fetch).to.not.have.been.called();
  });

  it('should throw and error if DPNS contract is not present at specific height and DPNS setttings were set', async () => {
    dpnsContractId = generateRandomIdentifier();
    dpnsContractBlockHeight = blockHeight;

    endBlockHandler = endBlockHandlerFactory(
      blockExecutionDBTransactionsMock,
      dataContractRepositoryMock,
      dpnsContractBlockHeight,
      dpnsContractId,
      loggerMock,
    );

    dataContractRepositoryMock.fetch.resolves(null);

    try {
      await endBlockHandler(request);
      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NoDPNSContractFoundError);
      expect(e.getContractId()).to.equal(dpnsContractId);
      expect(e.getHeight()).to.equal(blockHeight);

      expect(blockExecutionDBTransactionsMock.getTransaction).to.have.been.calledOnceWithExactly(
        'dataContract',
      );
      expect(dataContractRepositoryMock.fetch).to.have.been.calledOnceWithExactly(
        dpnsContractId, transaction,
      );
    }
  });
});
