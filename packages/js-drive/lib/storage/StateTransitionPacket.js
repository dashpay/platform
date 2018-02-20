const Schema = require('@dashevo/dash-schema');

module.exports = class StateTransitionPacket extends Schema.TransitionPacket {
  constructor(data) {
    super(data);

    Object.assign(this, data);
  }
};
