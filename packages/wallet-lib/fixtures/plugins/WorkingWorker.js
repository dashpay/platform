import Worker from '../../src/plugins/Worker.js';

class WorkingWorker extends Worker {
  constructor() {
    super({
      name: 'WorkingWorker',
      firstExecutionRequired: true,
      executeOnStart: true,
      dependencies: [
        'storage', 'walletId',
      ],
      workerIntervalTime: 500
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
export default WorkingWorker;
