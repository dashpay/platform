/**
 * @method store
 * @method fetch
 */
class CreditsDistributionPoolRepositoryMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.store = sinon.stub();
    this.fetch = sinon.stub();
  }
}

module.exports = CreditsDistributionPoolRepositoryMock;
