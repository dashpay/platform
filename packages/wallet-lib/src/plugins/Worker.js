const _ = require('lodash');
const StandardPlugin = require('./StandardPlugin');
const { WorkerFailedOnExecute, WorkerFailedOnStart } = require('../errors');

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

    this.state = {
      started: false,
      ready: false,
    };
  }

  async startWorker() {
    let payloadResult = null;
    const self = this;
    try {
      if (this.worker) this.stopWorker();
      // every minutes, check the pool
      this.worker = setInterval(this.execWorker.bind(self), this.workerIntervalTime);

      if (this.executeOnStart === true) {
        if (this.onStart) {
          payloadResult = await this.onStart();
        }
      }
      const eventType = `WORKER/${this.name.toUpperCase()}/STARTED`;
      this.parentEvents.emit(eventType, { type: eventType, payload: payloadResult });
      this.state.started = true;

      if (this.executeOnStart) await this.execWorker();
    } catch (err) {
      throw new WorkerFailedOnStart(this.name, err.message, err);
    }
  }

  stopWorker() {
    clearInterval(this.worker);
    this.worker = null;
    this.workerPass = 0;
    this.isWorkerRunning = false;
    const eventType = `WORKER/${this.name.toUpperCase()}/STOPPED`;
    this.state.started = false;
    this.parentEvents.emit(eventType, { type: eventType, payload: null });
  }

  async execWorker() {
    let payloadResult = null;
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
        payloadResult = await this.execute();
      } catch (err) {
        await this.stopWorker();
        throw new WorkerFailedOnExecute(this.name, err.message, err);
      }
    } else {
      throw new Error(`Worker ${this.name} : Missing execute function`);
    }

    this.isWorkerRunning = false;
    this.workerPass += 1;
    if (!this.state.ready) this.state.ready = true;
    const eventType = `WORKER/${this.name.toUpperCase()}/EXECUTED`;
    this.parentEvents.emit(eventType, { type: eventType, payload: payloadResult });
    return true;
  }
}

module.exports = Worker;
