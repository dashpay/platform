import {Block} from "@dashevo/dashcore-lib/typings/block/Block";
import {BlockHeader} from "@dashevo/dashcore-lib/typings/block/BlockHeader";
import {Network} from "../types/types";
import {Transaction} from "@dashevo/dashcore-lib/typings/transaction/Transaction";

export declare class BaseTransporter {
    constructor(props: BaseTransporterOptions);
    type: string;
    state:{
        block: any,
        blockHeaders: any,
        executors:{
            blocks: any,
            blockHeaders: any,
            addresses: any
        }
        addressTransactionsMap: any;
        subscriptions: {
            addresses: any
        }
    }

    announce(eventName: string, args: any): void;
    disconnect(): boolean;
    getAddressSummary(address: string): any;
    getBestBlock(): Promise<Block>;
    getBestBlockHeader(): Promise<BlockHeader>;
    getBestBlockHash(): Promise<string>;
    getBestBlockHeight(): Promise<number>;
    getBlockByHash(hash: string): Promise<Block>;
    getBlockByHeight(height: number): Promise<Block>;
    getBlockHeaderByHash(hash: string): Promise<BlockHeader>;
    getBlockHeaderByHeight(height: number): Promise<BlockHeader>;
    getStatus(height: number): Promise<{
        coreVersion: number,
        protocolVersion: number,
        blocks: number,
        timeOffset: number,
        connections: number,
        proxy: string,
        difficulty: number,
        testnet: false,
        relayFee: number,
        errors: string,
        network: Network,
    }>;
    getTransaction(hash: number): Promise<Transaction>;
    getUTXO(address: string):Promise<{
        totalItems: number,
        from: number,
        to: number,
        items: [
            {
                address: string,
                txid: string,
                outputIndex: number,
                script: string,
                satoshis: number,
                height: number,
            },
        ],
    }>;
    sendTransaction(rawtx: string): Promise<string>
    subscribeToAddressesTransactions(addressList:[string]): Promise<void>
    subscribeToBlockHeaders(): Promise<void>
    subscribeToBlocks(): Promise<void>
    getIdentityIdByFirstPublicKey(publicKeyHash: string): Promise<string|null>
}

export declare interface BaseTransporterOptions {
    type: string;
}
export interface Transporter extends BaseTransporter {}
