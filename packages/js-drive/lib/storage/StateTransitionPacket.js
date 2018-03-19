const Schema = require('@dashevo/dash-schema');

class StateTransitionPacket extends Schema.TransitionPacket {
  constructor(data) {
    super(data);

    Object.assign(this, data);
  }
}

module.exports = StateTransitionPacket;
