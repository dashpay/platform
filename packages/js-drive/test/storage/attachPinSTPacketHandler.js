const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);
use(sinonChai);

const Emitter = require('emittery');

const attachPinSTPacketHandler = require('../../lib/storage/attachPinSTPacketHandler');
const getTransitionHeaderFixtures = require('../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('attachPinSTPacketHandler', () => {
  let transitionHeaders;
  let ipfsAPIMock;
  let iterationEmitter;

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
    iterationEmitter = new Emitter();
  });

  it('should pin ST packets when new header will appear', async () => {
    const header = transitionHeaders[0];

    attachPinSTPacketHandler(ipfsAPIMock, iterationEmitter);

    await iterationEmitter.emitSerial('header', header);

    expect(ipfsAPIMock.pin.add).has.calledOnce();
    expect(ipfsAPIMock.pin.add).has.calledWith(header.getStorageHash(), { recursive: true });
  });
});
