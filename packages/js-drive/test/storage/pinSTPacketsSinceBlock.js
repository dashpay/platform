const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

use(sinonChai);

const StateTransitionHeaderIterator = require('../../lib/blockchain/StateTransitionHeaderIterator');
const getTransitionHeaderFixtures = require('../../lib/test/fixtures/getTransitionHeaderFixtures');
const pinSTPacketsSinceBlock = require('../../lib/storage/pinSTPacketsSinceBlock');

describe('pinSTPacketsSinceBlock', () => {
  let transitionHeaders;
  let ipfsAPIMock;
  let stateTransitionHeaderIteratorMock;
  let nextStab;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    transitionHeaders = getTransitionHeaderFixtures();

    // Mock IPFS API
    const sinonSandbox = this.sinon;
    class IpfsAPI {
      constructor() {
        this.pin = {
          add: sinonSandbox.stub(),
        };
      }
    }

    ipfsAPIMock = new IpfsAPI();

    // Mock StateTransitionHeaderIterator
    const blockIteratorMock = {
      rpcClient: {
        getTransitionHeader() {
        },
      },
    };
    stateTransitionHeaderIteratorMock = new StateTransitionHeaderIterator(blockIteratorMock);

    nextStab = this.sinon.stub(stateTransitionHeaderIteratorMock, 'next');
    let currentHeaderIndex = 0;
    nextStab.callsFake(() => {
      if (!transitionHeaders[currentHeaderIndex]) {
        return Promise.resolve({ done: true });
      }

      const currentHeader = transitionHeaders[currentHeaderIndex];

      currentHeaderIndex++;

      return Promise.resolve({ done: false, value: currentHeader });
    });
  });

  it('should pin ST packets by hash from ST headers from blockchain', async () => {
    await pinSTPacketsSinceBlock(ipfsAPIMock, stateTransitionHeaderIteratorMock);

    expect(nextStab).has.callCount(transitionHeaders.length + 1);

    expect(ipfsAPIMock.pin.add).has.callCount(transitionHeaders.length);

    transitionHeaders.forEach((header) => {
      expect(ipfsAPIMock.pin.add).to.be.calledWith(header.getStorageHash(), { recursive: true });
    });
  });
});
