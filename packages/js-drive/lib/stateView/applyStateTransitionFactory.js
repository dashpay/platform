const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');

const Reference = require('./revisions/Reference');

/**
 * @param {updateSVContract} updateSVContract
 * @param {updateSVDocument} updateSVDocument
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  updateSVContract,
  updateSVDocument,
) {
  /**
   * @typedef {Promise} applyStateTransition
   * @param {AbstractStateTransition} stateTransition
   * @param {string} blockHash
   * @param {number} blockHeight
   * @param {MongoDBTransaction} stateViewTransaction
   * @returns {Promise<Object>}
   */
  async function applyStateTransition(
    stateTransition,
    blockHash,
    blockHeight,
    stateViewTransaction,
  ) {
    let result = {};

    // eslint-disable-next-line default-case
    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT: {
        const dataContract = stateTransition.getDataContract();

        const reference = new Reference({
          blockHash,
          blockHeight,
          stHash: stateTransition.hash(),
          hash: dataContract.hash(),
        });

        const svContract = await updateSVContract(
          dataContract,
          reference,
          stateViewTransaction,
        );

        result = { svContract };

        break;
      }

      case stateTransitionTypes.DOCUMENTS: {
        const documents = stateTransition.getDocuments();

        for (const document of documents) {
          const reference = new Reference({
            blockHash,
            blockHeight,
            stHash: stateTransition.hash(),
            hash: document.hash(),
          });

          await updateSVDocument(
            document,
            reference,
            stateViewTransaction,
          );
        }

        break;
      }
    }

    return result;
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
