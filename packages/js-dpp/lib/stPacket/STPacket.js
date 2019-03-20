const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateItemsMerkleRoot = require('./calculateItemsMerkleRoot');
const calculateItemsHash = require('./calculateItemsHash');

const DPContract = require('../contract/DPContract');

const ContractAndDocumentsNotAllowedSamePacketError = require('./errors/ContractAndDocumentsNotAllowedSamePacketError');

class STPacket {
  /**
   * @param {string} contractId
   * @param {DPContract|Document[]} [items] DP Contract or Documents
   */
  constructor(contractId, items = undefined) {
    this.setDPContractId(contractId);

    this.documents = [];
    this.contracts = [];

    if (items instanceof DPContract) {
      this.setDPContract(items);
    }

    if (Array.isArray(items)) {
      this.setDocuments(items);
    }
  }

  /**
   * Set DP Contract ID
   *
   * @param {string} contractId
   */
  setDPContractId(contractId) {
    this.contractId = contractId;

    return this;
  }

  /**
   * Get DP Contract ID
   *
   * @return {string}
   */
  getDPContractId() {
    return this.contractId;
  }


  /**
   * Get items merkle root
   *
   * @return {string|null}
   */
  getItemsMerkleRoot() {
    return calculateItemsMerkleRoot({
      contracts: this.contracts,
      documents: this.documents,
    });
  }

  /**
   * Get items hash
   *
   * @return {string}
   */
  getItemsHash() {
    return calculateItemsHash({
      contracts: this.contracts,
      documents: this.documents,
    });
  }

  /**
   * Set DP Contract
   *
   * @param {DPContract} dpContract
   */
  setDPContract(dpContract) {
    if (this.documents.length > 0) {
      throw new ContractAndDocumentsNotAllowedSamePacketError(this);
    }

    this.contracts = !dpContract ? [] : [dpContract];

    return this;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract|null}
   */
  getDPContract() {
    if (this.contracts.length) {
      return this.contracts[0];
    }

    return null;
  }

  /**
   * Set Documents
   *
   * @param {Document[]} documents
   */
  setDocuments(documents) {
    if (this.contracts.length) {
      throw new ContractAndDocumentsNotAllowedSamePacketError(this);
    }

    this.documents = documents;

    return this;
  }

  /**
   * Get Documents
   *
   * @return {Document[]}
   */
  getDocuments() {
    return this.documents;
  }

  /**
   * Add Document
   *
   * @param {Document...} documents
   */
  addDocument(...documents) {
    this.documents.push(...documents);

    return this;
  }

  /**
   * Return ST Packet as plain object
   *
   * @return {{contractId: string,
   *           itemsMerkleRoot: string,
   *           itemsHash: string,
   *           contracts: {
   *               $schema: string,
   *               name: string,
   *               version: number,
   *               documents: Object<string, Object>,
   *               [definitions]: Object<string, Object>
   *           }[],
   *           documents: {
   *               $type: string,
   *               $scope: string,
   *               $scopeId: string,
   *               $rev: number,
   *               $action: number
   *           }[]}}
   */
  toJSON() {
    return {
      contractId: this.getDPContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
      contracts: this.contracts.map(dpContract => dpContract.toJSON()),
      documents: this.documents.map(document => document.toJSON()),
    };
  }

  /**
   * Return serialized ST Packet
   *
   * @return {Buffer}
   */
  serialize() {
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with ST packet hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
  }
}

module.exports = STPacket;
