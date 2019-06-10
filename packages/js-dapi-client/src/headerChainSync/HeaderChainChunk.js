class HeaderChainChunk {
  /**
   * @param {int} fromHeight
   * @param {int} size
   * @param {int} step
   */
  constructor(fromHeight, size, step) {
    this.fromHeight = fromHeight;
    this.size = size;
    this.step = step;
  }

  /**
   * Get starting height of the chunk
   *
   * @returns {int}
   */
  getFromHeight() {
    return this.fromHeight;
  }

  /**
   * Get size of the chunk
   *
   * @returns {int}
   */
  getSize() {
    return this.size;
  }

  /**
   * Get the download step
   *
   * @returns {int}
   */
  getStep() {
    return this.step;
  }

  /**
   * Get computed final height
   *
   * @returns {int}
   */
  getToHeight() {
    return this.fromHeight + this.size;
  }

  /**
   * Get computed extra headers size in case
   * if chunk size cannot be divided equally
   * by step size
   *
   * @returns {int}
   */
  getExtraSize() {
    return this.size % this.step;
  }
}

module.exports = HeaderChainChunk;
