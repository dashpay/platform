/**
 * @method setAmount
 * @method incrementAmount
 * @method getAmount
 * @method populate
 * @method toJSON
 */
class CreditsDistributionPoolMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.setAmount = sinon.stub();
    this.incrementAmount = sinon.stub();
    this.getAmount = sinon.stub();
    this.populate = sinon.stub();
    this.toJSON = sinon.stub();
  }
}

module.exports = CreditsDistributionPoolMock;
