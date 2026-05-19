import Worker from '../../src/plugins/Worker.js';

class FaultyWorker extends Worker {
  constructor() {
    super({
      name: 'FaultyWorker',
      firstExecutionRequired: true,
      executeOnStart: true,
      executeAfterStart: true,
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

export default FaultyWorker;
