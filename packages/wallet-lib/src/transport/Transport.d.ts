import {Block, BlockHeader, Transaction} from "@dashevo/dashcore-lib";
export declare interface Transport {
    announce(eventName: string, args: any[]): void

    disconnect(): void

    getBestBlock(): Promise<Block>

    getBestBlockHash(): Promise<string>

    getBestBlockHeader(): Promise<BlockHeader>

    getBestBlockHeight(): Promise<number>

    getBlockByHash(hash: string): Promise<Block>

    getBlockByHeight(height: number): Promise<Block>

    getBlockHeaderByHash(hash: string): Promise<BlockHeader>

    getBlockHeaderByHeight(height: number): Promise<BlockHeader>

    getIdentityByPublicKeyHash(publicKeyHash: Buffer): Promise<Buffer[]>

    getBlockchainStatus(): Promise<object>

    getTransaction(txid: string): Promise<Transaction>

    sendTransaction(serializedTransaction: string): Promise<string>

    subscribeToAddressesTransactions(): Promise<void>
}
