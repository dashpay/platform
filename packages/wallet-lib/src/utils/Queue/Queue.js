const Emitter = require('events').EventEmitter;

class Queue extends Emitter {
  constructor(options = { autoProcess: true }) {
    super();
    this.jobs = [];
    this.state = {
      isProcessing: false,
    };
    this.autoProcess = options.autoProcess || true;

    if (this.autoProcess) {
      this.startAutoProcessing();
    }
  }

  getSize() {
    return this.jobs.length;
  }

  enqueueJob(job) {
    this.jobs.push(job);
    this.emit('enqueued', job.id);
  }

  dequeueJob() {
    const job = this.jobs.shift();
    this.emit('dequeued', job);
    return job;
  }

  async processNext() {
    const job = this.dequeueJob();
    if (job) {
      await this.processJob(job);
    }
  }

  async processJob(job) {
    this.state.isProcessing = true;
    const result = await job.fn();
    this.state.isProcessing = false;
    this.emit('processed', { result, job });
    return result;
  }

  startAutoProcessing() {
    const self = this;

    const processOnEnqueuedEvent = async () => {
      try {
        await self.processNext();
      } catch (e) {
        self.emit('error', e);
      }
    };
    const processOnProcessedEvent = async () => {
      try {
        await self.processNext();
      } catch (e) {
        self.emit('error', e);
      }
      if (!self.getSize()) {
        this.once('enqueued', processOnEnqueuedEvent);
      }
    };

    this.on('processed', processOnProcessedEvent);
    this.once('enqueued', processOnEnqueuedEvent);
  }
}

module.exports = Queue;
