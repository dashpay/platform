const InvalidParamsError = require('../InvalidParamsError');
const InvalidWhereError = require('../../stateView/dapObject/errors/InvalidWhereError');
const InvalidOrderByError = require('../../stateView/dapObject/errors/InvalidOrderByError');
const InvalidLimitError = require('../../stateView/dapObject/errors/InvalidLimitError');
const InvalidStartAtError = require('../../stateView/dapObject/errors/InvalidStartAtError');
const InvalidStartAfterError = require('../../stateView/dapObject/errors/InvalidStartAfterError');
const AmbiguousStartError = require('../../stateView/dapObject/errors/AmbiguousStartError');

/**
 * @param fetchDapObjects
 * @returns {fetchDapObjectsMethod}
 */
module.exports = function fetchDapObjectsMethodFactory(fetchDapObjects) {
  /**
   * @typedef {Promise} fetchDapObjectsMethod
   * @param {{ dapId: string, type: string, options: object }} params
   * @returns {Promise<object[]>}
   */
  async function fetchDapObjectsMethod(params) {
    if (!params.dapId || !params.type) {
      throw new InvalidParamsError();
    }

    try {
      return fetchDapObjects(params.dapId, params.type, params.options);
    } catch (error) {
      switch (error.constructor) {
        case InvalidWhereError:
        case InvalidOrderByError:
        case InvalidLimitError:
        case InvalidStartAtError:
        case InvalidStartAfterError:
        case AmbiguousStartError:
          throw new InvalidParamsError();
        default:
          throw error;
      }
    }
  }

  return fetchDapObjectsMethod;
};
