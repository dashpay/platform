/**
 * @abstract
 */
class AbstractDocumentTransition {
  constructor(rawDocumentTransition) {
    this.dataContractId = rawDocumentTransition.$dataContractId;
  }

  /**
   * @abstract
   */
  getAction() {
    throw new Error('Not implemented');
  }

  /**
   * Get Data Contract ID
   *
   * @return {string}
   */
  getDataContractId() {
    return this.dataContractId;
  }

  /**
   * Get JSON representation
   *
   * @returns { { $action: string, $dataContractId: string } }
   */
  toJSON() {
    return {
      $action: this.getAction(),
      $dataContractId: this.getDataContractId(),
    };
  }
}

AbstractDocumentTransition.ACTIONS = {
  CREATE: 0,
  REPLACE: 1,
  // 2 reserved for UPDATE
  DELETE: 3,
};

AbstractDocumentTransition.ACTION_NAMES = {
  CREATE: 'create',
  REPLACE: 'replace',
  DELETE: 'delete',
};

module.exports = AbstractDocumentTransition;
