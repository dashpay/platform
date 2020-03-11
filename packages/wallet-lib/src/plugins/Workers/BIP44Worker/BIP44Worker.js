const { Worker } = require('../../');


// TODO : REfacto
class BIP44Worker extends Worker {
  constructor() {
    super({
      name: 'BIP44Worker',
      firstExecutionRequired: true,
      executeOnStart: true,
      dependencies: [
        'storage', 'getAddress', 'walletId', 'index', 'walletType',
      ],
    });
  }

  execute() {
    // Following BIP44 Account Discovery section, we will scan the external chain of this account.
    // We do not need to scan the internal as it's linked to external's one
    // So we just seek for 1:1 internal of external.
    this.ensureEnoughAddress();
  }
}

BIP44Worker.prototype.ensureEnoughAddress = require('./ensureEnoughAddress');

module.exports = BIP44Worker;
