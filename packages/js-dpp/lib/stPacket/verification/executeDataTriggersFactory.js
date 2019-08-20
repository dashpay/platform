/**
 * Execute data triggers for a document sequentially
 *
 * @param {Document} document
 * @param {DataTrigger[]} dataTriggers
 * @param {DataTriggerExecutionContext} context
 * @param {DataTriggerExecutionResult[]} results
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function executeTriggersSequentially(document, dataTriggers, context, results) {
  return dataTriggers.reduce(async (previousPromise, dataTrigger) => {
    const result = await previousPromise;
    if (result) {
      results.push(result);
    }
    return dataTrigger.execute(document, context);
  }, Promise.resolve()).then(lastResult => results.push(lastResult));
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
   * @typedef executeDataTriggers
   *
   * @param {Document[]} documents
   * @param {DataTriggerExecutionContext} context
   *
   * @return {Promise<DataTriggerExecutionResult[]>}
   */
  async function executeDataTriggers(documents, context) {
    const contractId = context.getContract().getId();

    const results = [];

    await documents.reduce(async (previousPromise, document) => {
      await previousPromise;

      const dataTriggers = getDataTriggers(contractId, document.getType(), document.getAction());

      return executeTriggersSequentially(document, dataTriggers, context, results);
    }, Promise.resolve());

    return results;
  }

  return executeDataTriggers;
}

module.exports = executeDataTriggersFactory;
