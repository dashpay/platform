const Emittery = require('emittery');
const BlockchainReaderState = require('../../blockchain/reader/BlockchainReaderState');

class BlockchainReaderMediatorMock extends Emittery {
  constructor(sinonSandbox) {
    super();

    this.state = sinonSandbox.stub(new BlockchainReaderState());
    this.reset = sinonSandbox.stub();
    this.getInitialBlockHeight = sinonSandbox.stub();

    const classMethods = Object.getPrototypeOf(this);
    const emitteryMethods = Object.getPrototypeOf(classMethods);

    this.emitSerial = sinonSandbox.stub();
    this.originalEmitSerial = emitteryMethods.emitSerial.bind(this);
  }

  getState() {
    return this.state;
  }
}

module.exports = BlockchainReaderMediatorMock;
