// TODO: It might be part of SDK in the future

const TransitionHeader = require('bitcore-lib-dash/lib/stateTransition/transitionHeader');

module.exports = class StateTransitionHeader extends TransitionHeader {
  constructor(data) {
    super(data);

    // TODO: Remove when getStorageHash will be implemented in bitcore-lib
    if (data.storageHash) {
      this.storageHash = data.storageHash;
      this.getStorageHash = function getStorageHash() {
        return this.storageHash;
      };
    }
  }
};
