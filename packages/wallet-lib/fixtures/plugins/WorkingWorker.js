const Worker = require('../../src/plugins/Worker');

class WorkingWorker extends Worker {
  constructor() {
    super({
      name: 'WorkingWorker',
      firstExecutionRequired: true,
      executeOnStart: true,
      dependencies: [
        'storage', 'walletId',
      ],
    });
  }

  execute() {
    const { storage } = this;
    if (storage.workingWorkerPass === undefined) {
      storage.workingWorkerPass = 0;
    }

    storage.workingWorkerPass += 1;
  }
}
module.exports = WorkingWorker;
