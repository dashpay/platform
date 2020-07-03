import {Block, BlockHeader, Transaction} from "@dashevo/dashcore-lib";

export declare interface Transport {
    announce(eventName, args)

    disconnect()

    getAddressSummary(address)

    getBestBlock(): Promise<Block>

    getBestBlockHash(): Promise<string>

    getBestBlockHeader(): Promise<BlockHeader>

    getBestBlockHeight(): Promise<number>

    getBlockByHash(hash): Promise<Block>

    getBlockByHeight(height): Promise<Block>

    getBlockHeaderByHash(hash): Promise<BlockHeader>

    getBlockHeaderByHeight(height): Promise<BlockHeader>

    getIdentityIdByFirstPublicKey(publicKeyHash): Promise<string>

    getStatus(): Promise<object>

    getTransaction(txid): Promise<Transaction>

    getUTXO(address): Promise<object>

    sendTransaction(serializedTransaction): Promise<string>

    subscribeToAddressesTransactions()

    subscribeToBlockHeaders()

    subscribeToBlocks()
}
