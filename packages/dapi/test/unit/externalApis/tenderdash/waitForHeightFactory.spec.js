const EventEmitter = require('events');

const waitForHeightFactory = require('../../../../lib/externalApis/tenderdash/waitForHeightFactory');
const BlockchainListener = require('../../../../lib/externalApis/tenderdash/BlockchainListener');

describe('waitForHeightFactory', () => {
  let blockchainListenerMock;
  let waitForHeight;
  let blockMessageMock;

  beforeEach(() => {
    blockchainListenerMock = new EventEmitter();

    blockMessageMock = {
      data: {
        value: {
          block: {
            header: {
              height: '123',
            },
            data: {
              txs: [],
            },
          },
        },
      },
    };

    waitForHeight = waitForHeightFactory(
      blockchainListenerMock,
    );
  });

  it('should resolve promise when the current block height is getting equal to specified height', () => {
    const promise = waitForHeight(123);

    blockchainListenerMock.emit(BlockchainListener.EVENTS.NEW_BLOCK, blockMessageMock);

    expect(promise).to.be.fulfilled();
  });

  it('should resolve promise if specified height is equal or higher than the current block height', () => {
    const promise = waitForHeight(120);

    expect(promise).to.be.fulfilled();
  });
});
