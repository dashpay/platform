const InvalidParamsError = require('../InvalidParamsError');
const InvalidWhereError = require('../../stateView/dapObject/InvalidWhereError');
const InvalidOrderByError = require('../../stateView/dapObject/InvalidOrderByError');
const InvalidLimitError = require('../../stateView/dapObject/InvalidLimitError');
const InvalidStartAtError = require('../../stateView/dapObject/InvalidStartAtError');
const InvalidStartAfterError = require('../../stateView/dapObject/InvalidStartAfterError');

/**
 * @param fetchDapObjects
 * @returns {fetchDapObjectsMethod}
 */
module.exports = function fetchDapObjectsMethodFactory(fetchDapObjects) {
  /**
   * @typedef {Promise} fetchDapObjectsMethod
   * @param {string} dapId
   * @param {string} type
   * @param {object} options
   * @returns {Promise<object[]>}
   */
  async function fetchDapObjectsMethod({ dapId, type, options } = {}) {
    if (!dapId || !type) {
      throw new InvalidParamsError();
    }

    try {
      const dapObjects = await fetchDapObjects(dapId, type, options);
      return dapObjects.map(dapObject => dapObject.toJSON());
    } catch (error) {
      switch (error.constructor) {
        case InvalidWhereError:
        case InvalidOrderByError:
        case InvalidLimitError:
        case InvalidStartAtError:
        case InvalidStartAfterError:
          throw new InvalidParamsError();
        default:
          throw error;
      }
    }
  }

  return fetchDapObjectsMethod;
};
