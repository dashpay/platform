const Worker = require('../../src/plugins/Worker');

class FaultyWorker extends Worker {
  constructor() {
    super({
      name: 'FaultyWorker',
      firstExecutionRequired: true,
      executeOnStart: true,
      dependencies: [
        'storage', 'walletId',
      ],
    });
  }

  // eslint-disable-next-line class-methods-use-this
  execute() {
    throw new Error('Some reason.');
  }
}

module.exports = FaultyWorker;
