const BlockchainReaderMediatorMock = require('../../../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../../../lib/blockchain/reader/BlockchainReaderMediator');
const RestartBlockchainReaderError = require('../../../../../lib/blockchain/reader/RestartBlockchainReaderError');

const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlockFixtures');

const attachBlockErrorHandler = require('../../../../../lib/blockchain/reader/eventHandlers/attachBlockErrorHandler');

describe('attachBlockErrorHandler', () => {
  let readerMediatorMock;
  let blocks;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);

    blocks = getBlockFixtures();
  });

  it('should do nothing if skipBlockWithErrors is disabled', async () => {
    attachBlockErrorHandler(
      readerMediatorMock,
      {
        skipBlockWithErrors: false,
      },
    );

    await readerMediatorMock.originalEmitSerial(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      { block: blocks[0] },
    );

    expect(readerMediatorMock.emitSerial).to.be.not.called();
  });

  it('should restart reader from the next firstBlock if skipBlockWithErrors is enabled', async () => {
    attachBlockErrorHandler(
      readerMediatorMock,
      {
        skipBlockWithErrors: true,
      },
    );

    let expectedError;
    try {
      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.BLOCK_ERROR,
        { block: blocks[0] },
      );
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.be.instanceOf(RestartBlockchainReaderError);
    expect(expectedError.getHeight()).to.be.equal(blocks[1].height);

    expect(readerMediatorMock.emitSerial).to.be.calledOnceWith(
      ReaderMediator.EVENTS.BLOCK_SKIP,
      blocks[0],
    );
  });

  it('should not restart reader if the current firstBlock is the last one', async () => {
    attachBlockErrorHandler(
      readerMediatorMock,
      {
        skipBlockWithErrors: true,
      },
    );

    await readerMediatorMock.originalEmitSerial(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      { block: blocks[3] },
    );

    expect(readerMediatorMock.emitSerial).to.be.calledOnceWith(
      ReaderMediator.EVENTS.BLOCK_SKIP,
      blocks[3],
    );
  });
});
