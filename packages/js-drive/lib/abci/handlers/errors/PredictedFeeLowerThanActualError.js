const DriveError = require('../../../errors/DriveError');

class PredictedFeeLowerThanActualError extends DriveError {
  /**
   * @param {number} predictedFee
   * @param {number} actualFee
   * @param {AbstractStateTransition} stateTransition
   */
  constructor(predictedFee, actualFee, stateTransition) {
    super(`Predicted fee ${predictedFee} is lower than actual fee ${actualFee}`);

    this.stateTransition = stateTransition;
  }

  getStateTransition() {
    return this.stateTransition;
  }
}

module.exports = PredictedFeeLowerThanActualError;
