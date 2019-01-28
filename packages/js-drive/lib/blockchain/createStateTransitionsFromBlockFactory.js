const StateTransition = require('./StateTransition');

/**
 * @param {RpcClient} rpcClient
 * @return {createStateTransitionsFromBlock}
 */
module.exports = function createStateTransitionsFromBlockFactory(rpcClient) {
  /**
   * Search for previous transition index in an array
   *
   * @param {StateTransition} referenceTransition
   * @param {StateTransition[]} transitions
   * @returns {number}
   */
  function findPrevTransitionIndex(referenceTransition, transitions) {
    for (const [index, transition] of transitions.entries()) {
      if (referenceTransition.extraPayload.hashPrevSubTx === transition.hash) {
        return index;
      }
    }
    return -1;
  }

  /**
   * Search for next transition index in an array
   *
   * @param {StateTransition} referenceTransition
   * @param {StateTransition[]} transitions
   * @returns {number}
   */
  function findNextTransitionIndex(referenceTransition, transitions) {
    for (const [index, transition] of transitions.entries()) {
      if (referenceTransition.hash === transition.extraPayload.hashPrevSubTx) {
        return index;
      }
    }
    return -1;
  }

  /**
   * Move transition from source array to target one,
   * pushing target array's elements by using splice
   *
   * @param {number} sourceIndex
   * @param {number} targetIndex
   * @param {StateTransition[]} sourceArray
   * @param {StateTransition[]} targetArray
   */
  function moveTransition(sourceIndex, targetIndex, sourceArray, targetArray) {
    const transition = sourceArray[sourceIndex];
    sourceArray.splice(sourceIndex, 1);
    targetArray.splice(targetIndex, 0, transition);
  }

  /**
   * Put previous transition before target transition,
   * or put it at zero index if target transition is there
   *
   * @param {number} sourceIndex
   * @param {number} targetIndex
   * @param {StateTransition[]} sourceArray
   * @param {StateTransition[]} targetArray
   */
  function putPrevTransitionInPlace(sourceIndex, targetIndex, sourceArray, targetArray) {
    if (sourceIndex !== -1) {
      if (targetIndex === 0) {
        moveTransition(sourceIndex, 0, sourceArray, targetArray);
      } else {
        moveTransition(sourceIndex, targetIndex - 1, sourceArray, targetArray);
      }
    }
  }

  /**
   * Put next transition after target one
   *
   * @param {number} sourceIndex
   * @param {number} targetIndex
   * @param {StateTransition[]} sourceArray
   * @param {StateTransition[]} targetArray
   */
  function putNextTransitionInPlace(sourceIndex, targetIndex, sourceArray, targetArray) {
    if (sourceIndex !== -1) {
      moveTransition(sourceIndex, targetIndex + 1, sourceArray, targetArray);
    }
  }

  /**
   *  Sort state transitions array using `hash` and `hashPrevSubTx`
   *
   * @param {StateTransition[]} stateTransitions
   * @returns {StateTransition[]}
   */
  function sortStateTransitions(stateTransitions) {
    const [firstStateTransition, ...restOfStateTransitions] = stateTransitions;

    // Add first element to begin with
    const result = [firstStateTransition];

    // Search function based on the fact that we have chained data in the array
    // with no gaps, except for first and last elements
    const searchAndApply = () => {
      const firstSortedTransition = result[0];
      const lastSortedTransition = result[result.length - 1];

      // Find previous transition for the first sorted transition
      const prevTransitionIndex = findPrevTransitionIndex(
        firstSortedTransition,
        restOfStateTransitions,
      );

      // Immediately put transition on its place so indices are changed for next transition search
      putPrevTransitionInPlace(prevTransitionIndex, 0, restOfStateTransitions, result);

      // Find next transition for the last sorted transition
      const nextTransitionIndex = findNextTransitionIndex(
        lastSortedTransition,
        restOfStateTransitions,
      );

      // Immediately put transition on its place so indices are changed for next transition search
      putNextTransitionInPlace(
        nextTransitionIndex,
        result.length - 1,
        restOfStateTransitions,
        result,
      );

      // Re-run search and apply function if we have elements to search through
      if (restOfStateTransitions.length > 0) {
        searchAndApply();
      }
    };

    // Run the first iteration of search and apply
    searchAndApply();

    return result;
  }

  /**
   * @typedef createStateTransitionsFromBlock
   * @param {object} block
   * @return {StateTransition[]}
   */
  async function createStateTransitionsFromBlock(block) {
    const stateTransitions = [];

    for (const transactionId of block.tx) {
      const { result: serializedTransaction } = await rpcClient.getRawTransaction(transactionId);

      const transaction = new StateTransition(serializedTransaction);

      if (transaction.isSpecialTransaction()
          && transaction.type === StateTransition.TYPES.TRANSACTION_SUBTX_TRANSITION) {
        stateTransitions.push(transaction);
      }
    }

    // Group transitions by `regTxId` first
    const regTxIdGroupedTransitions = stateTransitions.reduce((accumulator, transition) => ({
      ...accumulator,
      [transition.extraPayload.regTxId]: [
        ...(accumulator[transition.extraPayload.regTxId] || []),
        transition,
      ],
    }), {});

    // Then sort transitions in increasing order using `hashPrevSubTx`
    // Result should be flattened list of state transitions
    return Object.entries(regTxIdGroupedTransitions)
      .map(([, group]) => group)
      .reduce(
        (accumulator, nextGroup) => accumulator.concat(sortStateTransitions(nextGroup)),
        [],
      );
  }

  return createStateTransitionsFromBlock;
};
