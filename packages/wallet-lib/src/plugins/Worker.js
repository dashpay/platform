const _ = require('lodash');
const StandardPlugin = require('./StandardPlugin');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  workerIntervalTime: 10 * 1000,
  executeOnStart: false,
  firstExecutionRequired: false,
  workerMaxPass: null,
};

class Worker extends StandardPlugin {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    super({ type: 'Worker', ...opts });
    this.worker = null;
    this.workerPass = 0;
    this.isWorkerRunning = false;

    this.firstExecutionRequired = _.has(opts, 'firstExecutionRequired')
      ? opts.firstExecutionRequired
      : defaultOpts.firstExecutionRequired;

    this.executeOnStart = _.has(opts, 'executeOnStart')
      ? opts.executeOnStart
      : defaultOpts.executeOnStart;

    this.workerIntervalTime = (opts.workerIntervalTime)
      ? opts.workerIntervalTime
      : defaultOpts.workerIntervalTime;

    this.workerMaxPass = (opts.workerMaxPass)
      ? opts.workerMaxPass
      : defaultOpts.workerMaxPass;
  }

  async startWorker() {
    const self = this;
    if (this.worker) this.stopWorker();
    // every minutes, check the pool
    this.worker = setInterval(this.execWorker.bind(self), this.workerIntervalTime);
    if (this.executeOnStart === true) {
      await this.execWorker();
    }
    this.events.emit(`WORKER/${this.name.toUpperCase()}/STARTED`);
  }

  stopWorker() {
    clearInterval(this.worker);
    this.worker = null;
    this.workerPass = 0;
    this.isWorkerRunning = false;
    this.events.emit(`WORKER/${this.name.toUpperCase()}/STOPPED`);
  }

  async execWorker() {
    if (this.isWorkerRunning) {
      return false;
    }
    if (this.workerMaxPass !== null && this.workerPass >= this.workerMaxPass) {
      this.stopWorker();
      return false;
    }
    this.isWorkerRunning = true;

    if (this.execute) {
      try {
        await this.execute();
      } catch (e) {
        console.log(e);
      }
    } else {
      console.error(`${this.name} : Missing execute function`);
    }

    this.isWorkerRunning = false;
    this.workerPass += 1;

    this.events.emit(`WORKER/${this.name.toUpperCase()}/EXECUTED`);
    return true;
  }
}
module.exports = Worker;
