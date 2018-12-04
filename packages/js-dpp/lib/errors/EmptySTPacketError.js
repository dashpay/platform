const ConsensusError = require('./ConsensusError');

class EmptySTPacketError extends ConsensusError {
  constructor() {
    super('ST Packet is empty');
  }
}

module.exports = EmptySTPacketError;
