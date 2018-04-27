const TransitionHeader = require('@dashevo/dashcore-lib/lib/stateTransition/transitionHeader');

class StateTransitionHeader extends TransitionHeader {
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
}

module.exports = StateTransitionHeader;
