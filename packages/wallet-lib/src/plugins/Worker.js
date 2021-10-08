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

    this.awaitOnInjection = _.has(opts, 'awaitOnInjection')
      ? opts.awaitOnInjection
      : false;

    this.firstExecutionRequired = _.has(opts, 'firstExecutionRequired')
      ? opts.firstExecutionRequired
      : defaultOpts.firstExecutionRequired;

    this.executeOnStart = _.has(opts, 'executeOnStart')
      ? opts.executeOnStart
      : defaultOpts.executeOnStart;

    this.workerIntervalTime = _.has(opts, 'workerIntervalTime')
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
    const eventTypeStarting = `WORKER/${this.name.toUpperCase()}/STARTING`;
    logger.debug(JSON.stringify({ eventTypeStarting, result: payloadResult }));
    this.parentEvents.emit(eventTypeStarting, { type: eventTypeStarting, payload: payloadResult });
    try {
      if (this.worker) await this.stopWorker();

      if (this.workerIntervalTime > 0) {
        this.worker = setInterval(this.execWorker.bind(self), this.workerIntervalTime);
      }

      if (this.executeOnStart === true) {
        if (this.onStart) {
          payloadResult = await this.onStart();
        }
      }
      const eventTypeStarted = `WORKER/${this.name.toUpperCase()}/STARTED`;
      logger.debug(JSON.stringify({ eventTypeStarted, result: payloadResult }));
      this.parentEvents.emit(eventTypeStarted, { type: eventTypeStarted, payload: payloadResult });
      this.state.started = true;

      if (this.executeOnStart) await this.execWorker();
    } catch (e) {
      this.emit('error', e, {
        type: 'plugin',
        pluginType: 'worker',
        pluginName: this.name,
      });
    }
  }

  async stopWorker(reason = null) {
    let payloadResult = reason;
    clearInterval(this.worker);
    this.worker = null;
    this.workerPass = 0;
    this.isWorkerRunning = false;
    const eventType = `WORKER/${this.name.toUpperCase()}/STOPPED`;
    if (this.onStop) {
      payloadResult = await this.onStop();
    }
    this.state.started = false;
    logger.debug(JSON.stringify({ eventType, result: payloadResult }));
    this.parentEvents.emit(eventType, { type: eventType, payload: payloadResult });
  }

  async execWorker() {
    let payloadResult = null;
    if (this.isWorkerRunning) {
      return false;
    }
    if (this.workerMaxPass !== null && this.workerPass >= this.workerMaxPass) {
      await this.stopWorker();
      return false;
    }
    this.isWorkerRunning = true;

    if (this.execute) {
      try {
        payloadResult = await this.execute();
      } catch (e) {
        await this.stopWorker(e.message);
        this.emit('error', e, {
          type: 'plugin',
          pluginType: 'worker',
          pluginName: this.name,
        });
      }
    } else {
      throw new Error(`Worker ${this.name}: Missing execute function`);
    }

    this.isWorkerRunning = false;
    this.workerPass += 1;
    if (!this.state.ready) this.state.ready = true;
    const eventType = `WORKER/${this.name.toUpperCase()}/EXECUTED`;
    logger.debug(JSON.stringify({ eventType, result: payloadResult }));
    this.parentEvents.emit(eventType, { type: eventType, payload: payloadResult });
    return true;
  }
}

module.exports = Worker;
