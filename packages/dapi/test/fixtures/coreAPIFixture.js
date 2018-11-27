module.exports = {
  async estimateFee(numberOfBlocks) { return 1; },
  async getAddressSummary(address) { return {}; },
  async getAddressTotalReceived(address) { return 1000; },
  async getAddressTotalSent(address) { return 900; },
  async getAddressUnconfirmedBalance(address) { return 1100; },
  async getBalance(address) { return 100; },
  async getBestBlockHeight() { return 243789; },
  async getBlockHash() { return 'hash'; },
  async getBlockHeaders() { return [{}]; },
  async getBlockHeader() { return {}; },
  async getBlocks(blockDate, limit) { return [{}]; },
  async getHistoricBlockchainDataSyncStatus() {
    return {};
  },
  async getMasternodesList() { return [{ ip: '127.0.0.1' }]; },
  async getPeerDataSyncStatus() { return ''; },
  async getMnListDiff() {
    return {
      baseBlockHash: '0000000000000000000000000000000000000000000000000000000000000000',
      blockHash: '0000000000000000000000000000000000000000000000000000000000000000',
      deletedMNs: [],
      mnList: [],
      merkleRootMNList: '0000000000000000000000000000000000000000000000000000000000000000',
    };
  },
  async getRawBlock(blockHash) { return {}; },
  async getStatus(query) { return {}; },
  async getTransactionById(txid) { return {}; },
  async getTransactionsByAddress(address) { return []; },
  async getUser(usernameOrUserId) { return {}; },
  async getUTXO(address) { return []; },
  async sendRawTransaction(rawTransaction) { return 'txid'; },
  async sendRawIxTransaction(rawTransaction) { return 'txid'; },
  async generate(amount) { return new Array(amount); },
  async sendRawTransition(rawTransitionHeader) { return 'tsid'; },
  // Todo: not yet final spec so it may change
  async getQuorum() {
    return {
      quorum: [
        {
          proRegTxHash: '3450cdbaa92432dd19672738342cb4f2467f1a8b142c31142ea39e14f3ab8c18',
          service: '165.227.144.38:19999',
          keyIDOperator: 'e6be850bfe045d2cd2b0e5789010b1a910dd7d27',
          keyIDVoting: 'e6be850bfe045d2cd2b0e5789010b1a910dd7d27',
          isValid: true,
        },
        {
          proRegTxHash: '47b3adaa8ed42c6c67abb317e631cf674381cd8fd87033bcb92f3e2d21d08360',
          service: '159.89.110.184:19999',
          keyIDOperator: '4d5fce2325deb034ae75a625a3e2f09395e27bf7',
          keyIDVoting: '4d5fce2325deb034ae75a625a3e2f09395e27bf7',
          isValid: true,
        },
        {
          proRegTxHash: '049d0c6dd63bb50c0bfee9106ad7ce5f9b4e9ef4487552cd4638317b3b05ffee',
          service: '142.93.170.82:19999',
          keyIDOperator: 'cfdee11fc2b4ebf6e1cafb262269de4919698942',
          keyIDVoting: 'cfdee11fc2b4ebf6e1cafb262269de4919698942',
          isValid: true,
        },
      ],

      proofs: {
        merkleHashes: ['71e9bc59632243e13f2d4c463296cd5a7737beb397799fc5fc9ada93b69bf48c'],
        merkleFlags: 0x1d,
        blockHash: 'b5d2cd463831d63b7b3b05f0c0bfefee7ce5f9b4e9ef448755e049d0c6d9106a',
        totalTransactions: 1,
      },
      // todo after dashcore-lib specialtx
      quorumCommitmentTransaction: {
        quorumHash: 'd63bb5d2cd4638317b3b05f0c0bfee049d0c6d9106afee7ce5f9b4e9ef448755',
        prop1: '',
        prop2: '',
        prop3: '',
        prop4: '',
      },
    };
  },
};
