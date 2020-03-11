const _ = require('lodash');
const logger = require('../logger');
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

    this.state = {
      started: false,
      ready: false,
    };
  }

  async startWorker() {
    const self = this;
    if (this.worker) this.stopWorker();
    // every minutes, check the pool
    this.worker = setInterval(this.execWorker.bind(self), this.workerIntervalTime);
    if (this.executeOnStart === true) {
      if (this.onStart) {
        await this.onStart();
      }
    }
    const eventType = `WORKER/${this.name.toUpperCase()}/STARTED`;
    self.parentEvents.emit(eventType, { type: eventType, payload: null });
    self.state.started = true;
    if (this.executeOnStart) await this.execWorker();
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
      } catch (err) {
        logger.error(`${this.name} Error`, err);
      }
    } else {
      logger.error(`${this.name} : Missing execute function`);
    }

    this.isWorkerRunning = false;
    this.workerPass += 1;
    if (!this.state.ready) this.state.ready = true;
    const eventType = `WORKER/${this.name.toUpperCase()}/EXECUTED`;
    this.parentEvents.emit(eventType, { type: eventType, payload: null });
    return true;
  }
}
module.exports = Worker;
