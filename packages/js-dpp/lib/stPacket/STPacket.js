const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateItemsMerkleRoot = require('./calculateItemsMerkleRoot');
const calculateItemsHash = require('./calculateItemsHash');

const Contract = require('../contract/Contract');

const ContractAndDocumentsNotAllowedSamePacketError = require('./errors/ContractAndDocumentsNotAllowedSamePacketError');

class STPacket {
  /**
   * @param {string} contractId
   * @param {Contract|Document[]} [items] Contract or Documents
   */
  constructor(contractId, items = undefined) {
    this.setContractId(contractId);

    this.documents = [];
    this.contracts = [];

    if (items instanceof Contract) {
      this.setContract(items);
    }

    if (Array.isArray(items)) {
      this.setDocuments(items);
    }
  }

  /**
   * Set Contract ID
   *
   * @param {string} contractId
   */
  setContractId(contractId) {
    this.contractId = contractId;

    return this;
  }

  /**
   * Get Contract ID
   *
   * @return {string}
   */
  getContractId() {
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
   * Set Contract
   *
   * @param {Contract} contract
   */
  setContract(contract) {
    if (this.documents.length > 0) {
      throw new ContractAndDocumentsNotAllowedSamePacketError(this);
    }

    this.contracts = !contract ? [] : [contract];

    return this;
  }

  /**
   * Get Contract
   *
   * @return {Contract|null}
   */
  getContract() {
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
   * @return {RawSTPacket}
   */
  toJSON() {
    return {
      contractId: this.getContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
      contracts: this.contracts.map(contract => contract.toJSON()),
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
    return hash(this.serialize()).toString('hex');
  }
}

module.exports = STPacket;
