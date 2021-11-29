const AbstractStateTransitionIdentitySigned = require('../../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');

const AbstractDocumentTransition = require('./documentTransition/AbstractDocumentTransition');
const DocumentCreateTransition = require('./documentTransition/DocumentCreateTransition');
const DocumentReplaceTransition = require('./documentTransition/DocumentReplaceTransition');
const DocumentDeleteTransition = require('./documentTransition/DocumentDeleteTransition');

const IdentityPublicKey = require('../../../identity/IdentityPublicKey');

const actionsToClasses = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: DocumentCreateTransition,
  [AbstractDocumentTransition.ACTIONS.REPLACE]: DocumentReplaceTransition,
  [AbstractDocumentTransition.ACTIONS.DELETE]: DocumentDeleteTransition,
};

const Identifier = require('../../../identifier/Identifier');

class DocumentsBatchTransition extends AbstractStateTransitionIdentitySigned {
  /**
   * @param {RawDocumentsBatchTransition} [rawStateTransition]
   * @param {DataContract[]} dataContracts
   */
  constructor(rawStateTransition = {}, dataContracts) {
    super(rawStateTransition);

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'ownerId')) {
      this.ownerId = Identifier.from(rawStateTransition.ownerId);
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'transitions')) {
      const dataContractsMap = dataContracts.reduce((map, dataContract) => ({
        ...map,
        [dataContract.getId().toString('hex')]: dataContract,
      }), {});

      this.transitions = rawStateTransition.transitions.map((rawDocumentTransition) => (
        new actionsToClasses[rawDocumentTransition.$action](
          rawDocumentTransition,
          dataContractsMap[rawDocumentTransition.$dataContractId.toString('hex')],
        )
      ));
    }
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.DOCUMENTS_BATCH;
  }

  /**
   * Get owner id
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * Get document action transitions
   *
   * @return {DocumentCreateTransition[]|DocumentReplaceTransition[]|DocumentDeleteTransition[]}
   */
  getTransitions() {
    return this.transitions;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawDocumentsBatchTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawDocumentsBatchTransition = {
      ...super.toObject(options),
      ownerId: this.getOwnerId(),
      transitions: this.getTransitions().map((t) => t.toObject()),
    };

    if (!options.skipIdentifiersConversion) {
      rawDocumentsBatchTransition.ownerId = this.getOwnerId().toBuffer();
    }

    return rawDocumentsBatchTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonDocumentsBatchTransition}
   */
  toJSON() {
    const jsonStateTransition = {
      ...super.toJSON(),
      ownerId: this.getOwnerId().toString(),
    };

    // overwrite plain object transitions
    jsonStateTransition.transitions = this.getTransitions().map((t) => t.toJSON());

    return jsonStateTransition;
  }

  /**
   * Returns ids of all affected documents
   *
   * @return {Identifier[]}
   */
  getModifiedDataIds() {
    return this.getTransitions().map((documentTransition) => documentTransition.getId());
  }

  /**
   * Returns minimal key security level that can be used to sign this ST
   *
   * @override
   * @return {number}
   */
  getKeySecurityLevelRequirement() {
    const defaultSecurityLevel = IdentityPublicKey.SECURITY_LEVELS.HIGH;

    // Step 1: Get all document types for the ST
    // Step 2: Get document schema for every type
    // If schema has security level, use that, if not, use the default security level
    // Find the highest level (lowest int value) of all documents - the ST's signature
    // requirement is the highest level across all documents affected by the ST.
    const documentTransitions = this.getTransitions();
    let highestSecurityLevel;
    documentTransitions.forEach((documentTransition) => {
      const documentType = documentTransition.getType();
      const dataContract = documentTransition.getDataContract();
      const documentSchema = dataContract.getDocumentSchema(documentType);

      const documentKeySecurityLevelRequirement = documentSchema.keySecurityLevelRequirement == null
        ? defaultSecurityLevel
        : documentSchema.keySecurityLevelRequirement;

      if (
        highestSecurityLevel == null || highestSecurityLevel > documentKeySecurityLevelRequirement
      ) {
        highestSecurityLevel = documentKeySecurityLevelRequirement;
      }
    });

    return highestSecurityLevel;
  }
}

/**
 * @typedef {RawStateTransitionIdentitySigned & Object} RawDocumentsBatchTransition
 * @property {Buffer} ownerId
 * @property {
 *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
 * } transitions
 */

/**
 * @typedef {JsonStateTransitionIdentitySigned & Object} JsonDocumentsBatchTransition
 * @property {string} ownerId
 * @property {
 *   Array.<JsonDocumentCreateTransition|JsonDocumentReplaceTransition|JsonDocumentDeleteTransition>
 * } transitions
 */

module.exports = DocumentsBatchTransition;
