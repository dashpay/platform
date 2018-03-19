const EventEmitter = require('events');

class SyncStateRepositoryChangeListener extends EventEmitter {
  /**
   *
   * @param {SyncStateRepository} repository
   * @param {number} checkInterval Check interval in milliseconds
   */
  constructor(repository, checkInterval = 10000) {
    super();

    this.repository = repository;
    this.checkInterval = checkInterval;
    this.interval = null;
    this.previousState = null;
  }

  /**
   * Get repository
   *
   * @return {SyncStateRepository}
   */
  getRepository() {
    return this.repository;
  }

  /**
   * Listen changes and emit 'change' when sync state is updated
   *
   * @return {boolean}
   */
  async listen() {
    if (this.interval) {
      return false;
    }

    this.previousState = await this.repository.fetch();
    this.interval = setInterval(
      this.check.bind(this),
      this.checkInterval,
    );

    return true;
  }

  /**
   * Stop listener
   */
  stop() {
    clearInterval(this.interval);
    this.interval = null;
  }

  /**
   * @private
   */
  check() {
    if (!this.interval) {
      return;
    }

    this.repository.fetch().then((state) => {
      if (!state.isEqual(this.previousState)) {
        this.emit('change', state);
      }
    }).catch((e) => {
      this.emit('error', e);
    });
  }
}

module.exports = SyncStateRepositoryChangeListener;
