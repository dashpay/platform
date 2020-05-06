declare module "@dashevo/dapi-client" {
    /**
     * @param options - DAPI client options
     * @param {Array<Object>} [options.seeds] - seeds. If no seeds provided
     * default seed will be used.
     * @param {number} [options.port=3000] - default port for connection to the DAPI
     * @param {number} [options.nativeGrpcPort=3010] - Native GRPC port for connection to the DAPI
     * @param {number} [options.timeout=2000] - timeout for connection to the DAPI
     * @param {number} [options.retries=3] - num of retries if there is no response from DAPI node
     */

    /**
     * Class for DAPI Client
     */
    export default class DAPIClient {
        /**
         * Construct an instance of DAPI client
         */
        constructor(options: { retries: number; seeds: [string] | { service: string }[]; timeout: number; network: string });

        /**
         * Estimate fee
         * @param {number} numberOfBlocksToWait
         * @return {Promise<number>} - duffs per byte
         */
        estimateFee(numberOfBlocksToWait: number): Promise<number>;

        /**
         * ONLY FOR TESTING PURPOSES WITH REGTEST. WILL NOT WORK ON TESTNET/LIVENET.
         * @param {number} amount - Number of blocks to generate
         * @returns {Promise<string[]>} - block hashes
         */
        generate(amount: number): Promise<string[]>;

        /**
         * Returns a summary (balance, txs) for a given address
         * @param {string|string[]} address or array of addresses
         * @param {boolean} [noTxList=false] - true if a list of all txs should NOT be included in result
         * @param {number} [from] - start of range for the tx to be included in the tx list
         * @param {number} [to] - end of range for the tx to be included in the tx list
         * @param {number} [fromHeight] - which height to start from (optional, overriding from/to)
         * @param {number} [toHeight] - on which height to end (optional, overriding from/to)
         * @returns {Promise<Object>} - an object with basic address info
         */
        getAddressSummary(address: string | string[], noTxList?: boolean, from?: number, to?: number, fromHeight?: number, toHeight?: number): Promise<object>;

        /**
         * @param {string|string[]} address or array of addresses
         * @return {Promise<number>}
         */
        getAddressTotalSent(address: string | string[]): Promise<number>;

        /**
         * @param {string|string[]} address or array of addresses
         * @return {Promise<number>}
         */
        getAddressUnconfirmedBalance(address: string | string[]): Promise<number>;

        /**
         * @param {string|string[]} address or array of addresses
         * @return {Promise<number>}
         */
        getAddressTotalReceived(address: string | string[]): Promise<number>;

        /**
         * Returns balance for a given address
         * @param {string|string[]} address or array of addresses
         * @returns {Promise<number>} - address balance
         */
        getBalance(address: string | string[]): Promise<number>;

        /**
         * Returns block hash of chaintip
         * @returns {Promise<string>}
         */
        getBestBlockHash(): Promise<string>;

        /**
         * Returns best block height
         * @returns {Promise<number>}
         */
        getBestBlockHeight(): Promise<number>;

        /**
         * Returns block hash for the given height
         * @param {number} height
         * @returns {Promise<string>} - block hash
         */
        getBlockHash(height: number): Promise<string>;

        /**
         * Returns block header by hash
         * @param {string} blockHash
         * @returns {Promise<object[]>} - array of header objects
         */
        getBlockHeader(blockHash: string): Promise<object[]>;

        /**
         * Returns block headers from [offset] with length [limit], where limit is <= 2000
         * @param {number} offset
         * @param {number} [limit=1]
         * @param {boolean} [verbose=false]
         * @returns {Promise<object[]>} - array of header objects
         */
        getBlockHeaders(offset: number, limit?: number, verbose?: boolean): Promise<object[]>;

        /**
         * Get block summaries for the day
         * @param {string} blockDate string in format 'YYYY-MM-DD'
         * @param {number} limit
         * @return {Promise<object>}
         */
        getBlocks(blockDate: string, limit: number): Promise<object>;

        /**
         * @return {Promise<object>}
         */
        getHistoricBlockchainDataSyncStatus(): Promise<object>;

        /**
         * Retrieve user's last state transition hash
         *
         * @param {string} userId
         * @returns {Promise<string>}
         */
        getLastUserStateTransitionHash(userId: string): Promise<string>;

        /**
         * Returns mempool usage info
         * @returns {Promise<object>}
         */
        getMempoolInfo(): Promise<object>;

        /**
         * Get deterministic masternodelist diff
         * @param {string} baseBlockHash - hash or height of start block
         * @param {string} blockHash - hash or height of end block
         * @return {Promise<object>}
         */
        getMnListDiff(baseBlockHash: string, blockHash: string): Promise<object>;

        /**
         * @param {string} blockHash
         * @return {Promise<object>}
         */
        getRawBlock(blockHash: string): Promise<object>;

        /**
         * Returns Transactions for a given address or multiple addresses
         * @param {string|string[]} address or array of addresses
         * @param {number} [from] - start of range in the ordered list of latest UTXO (optional)
         * @param {number} [to] - end of range in the ordered list of latest UTXO (optional)
         * @param {number} [fromHeight] - which height to start from (optional, overriding from/to)
         * @param {number} [toHeight] - on which height to end (optional, overriding from/to)
         * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
         */
        getTransactionsByAddress(address: string | string[], from?: number, to?: number, fromHeight?: number, toHeight?: number): Promise<object>;

        /**
         * @param {string} txid - transaction hash
         * @return {Promise<object>}
         */
        getTransactionById(txid: string): Promise<object>;

        /**
         * Returns UTXO for a given address or multiple addresses (max result 1000)
         * @param {string|string[]} address or array of addresses
         * @param {number} [from] - start of range in the ordered list of latest UTXO (optional)
         * @param {number} [to] - end of range in the ordered list of latest UTXO (optional)
         * @param {number} [fromHeight] - which height to start from (optional, overriding from/to)
         * @param {number} [toHeight] - on which height to end (optional, overriding from/to)
         * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
         */
        getUTXO(address: string | string[], from?: number, to?: number, fromHeight?: number, toHeight?: number): Promise<object>;

        /**
         * @param {string} rawIxTransaction - hex-serialized instasend transaction
         * @return {Promise<string>} - transaction id
         */
        sendRawIxTransaction(rawIxTransaction: string): Promise<string>;

        /**
         * Sends serialized transaction to the network
         * @param {string} rawTransaction - hex string representing serialized transaction
         * @returns {Promise<string>} - transaction id
         */
        sendRawTransaction(rawTransaction: string): Promise<string>;

        /**
         * Fetch DAP Objects from DashDrive State View
         * @param {string} contractId
         * @param {string} type - Dap objects type to fetch
         * @param options
         * @param {Object} options.where - Mongo-like query
         * @param {Object} options.orderBy - Mongo-like sort field
         * @param {number} options.limit - how many objects to fetch
         * @param {number} options.startAt - number of objects to skip
         * @param {number} options.startAfter - exclusive skip
         * @return {Promise<Object[]>}
         */
        fetchDocuments(contractId: string, type: string, options: {
            where: any;
            orderBy: any;
            limit: number;
            startAt: number;
            startAfter: number;
        }): Promise<object[]>;

        /**
         * Returns blockchain user by its username or regtx id
         * @param {string} userId - user reg tx id
         * @returns {Promise<Object>} - blockchain user
         */
        getUserById(userId: string): Promise<object>;

        /**
         * Returns blockchain user by its username or regtx id
         * @param {string} username
         * @returns {Promise<Object>} - blockchain user
         */
        getUserByName(username: string): Promise<object>;

        /**
         * Sends serialized state transition header and data packet
         * @param {string} rawStateTransition - hex string representing state transition header
         * @param {string} rawSTPacket - hex string representing state transition data
         * @returns {Promise<string>} - header id
         */
        sendRawTransition(rawStateTransition: string, rawSTPacket: string): Promise<string>;

        /**
         * @param {Object} bloomFilter
         * @param {Uint8Array|string} bloomFilter.vData - The filter itself is simply a bit
         * field of arbitrary byte-aligned size. The maximum size is 36,000 bytes.
         * @param {number} bloomFilter.nHashFuncs - The number of hash functions to use in this filter.
         * The maximum value allowed in this field is 50.
         * @param {number} bloomFilter.nTweak - A random value to add to the seed value in the
         * hash function used by the bloom filter.
         * @param {number} bloomFilter.nFlags - A set of flags that control how matched items
         * are added to the filter.
         * @param {Object} [options]
         * @param {string} [options.fromBlockHash] - Specifies block hash to start syncing from
         * @param {number} [options.fromBlockHeight] - Specifies block height to start syncing from
         * @param {number} [options.count=0] - Number of blocks to sync, if set to 0 syncing is continuously
         * sends new data as well
         * @returns {
         *    Promise<EventEmitter>|!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>
         * }
         */
        subscribeToTransactionsWithProofs(bloomFilter: {
            vData: Uint8Array | string;
            nHashFuncs: number;
            nTweak: number;
            nFlags: number;
        }, options?: {
            fromBlockHash?: string;
            fromBlockHeight?: number;
            count?: number;
        }): Promise<any>;
    }
}