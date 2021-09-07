/**
 * Execute data triggers for a document sequentially
 *
 * @param {DocumentCreateTransition[]
 *        |DocumentReplaceTransition[]
 *        |DocumentDeleteTransition[]} documentTransition
 * @param {DataTrigger[]} dataTriggers
 * @param {DataTriggerExecutionContext} context
 * @param {DataTriggerExecutionResult[]} results
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function executeTriggersSequentially(documentTransition, dataTriggers, context, results) {
  return dataTriggers.reduce(async (previousPromise, dataTrigger) => {
    const result = await previousPromise;
    if (result) {
      results.push(result);
    }
    return dataTrigger.execute(documentTransition, context);
  }, Promise.resolve()).then((lastResult) => results.push(lastResult));
}

/**
 * Execute data trigger for a document with a context (factory)
 *
 * @param {getDataTriggers} getDataTriggers
 *
 * @return {executeDataTriggers}
 */
function executeDataTriggersFactory(getDataTriggers) {
  /**
   * Execute data trigger for a document with a context
   *
   * @typedef {executeDataTriggers}
   *
   * @param {DocumentCreateTransition[]
   *        |DocumentReplaceTransition[]
   *        |DocumentDeleteTransition[]} documentTransitions
   * @param {DataTriggerExecutionContext} context
   *
   * @return {DataTriggerExecutionResult[]}
   */
  async function executeDataTriggers(documentTransitions, context) {
    const dataContractId = context.getDataContract().getId();

    const results = [];

    await documentTransitions.reduce(async (previousPromise, documentTransition) => {
      await previousPromise;

      const dataTriggers = getDataTriggers(
        dataContractId,
        documentTransition.getType(),
        documentTransition.getAction(),
      );

      if (dataTriggers.length === 0) {
        return Promise.resolve();
      }

      return executeTriggersSequentially(documentTransition, dataTriggers, context, results);
    }, Promise.resolve());

    return results;
  }

  return executeDataTriggers;
}

module.exports = executeDataTriggersFactory;
