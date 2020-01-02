import {Mnemonic, Transaction, AddressObj,AddressInfo, AddressType, transactionId, TransactionInfo, PublicAddress, PrivateKey, Strategy, Network, Plugins} from "../types";
import {KeyChain} from "../KeyChain/KeyChain";
import {HDPrivateKey} from "@dashevo/dashcore-lib";

export declare class Account {
    constructor(options?: Account.Options);
    index: number;
    keyChain: KeyChain;
    state:any;

    isReady(): Promise<boolean>;
    isInitialized(): Promise<boolean>;
    broadcastTransaction(rawtx: string, isIS?: boolean): Promise<transactionId>;
    connect(): boolean;
    createTransaction(opts: Account.createTransactionOptions): Transaction;
    disconnect(): boolean;
    fetchAddressInfo(addresObj: AddressObj, fetchUtxo?:boolean): Promise<AddressInfo|false>;
    fetchStatus(): Promise<object|false>
    fetchTransactionInfo(transactionid: transactionId): Promise<TransactionInfo|false>
    forceRefreshAccount(): boolean;
    generateAddress(path:string): AddressObj;
    getAddress(_type: AddressType): AddressObj;
    getAddresses(rawtx: string, isIS: boolean): [AddressObj];
    getTotalBalance(displayDuffs?:boolean): number;
    getConfirmedBalance(displayDuffs?:boolean): number;
    getUnconfirmedBalance(displayDuffs?:boolean): number;
    getBIP44Path(network?:Network, index?:number): string;

    getDPA(name: string): object;

    getIdentityHDKey(index: number, identityType: string): HDPrivateKey;

    getNetwork(): Network;

    getPlugin(name:string): object;

    getPrivateKeys(addressList:[PublicAddress]): [PrivateKey];
    getTransaction(txid: transactionId): Transaction;
    getTransactionHistory(): [object];
    getTransactions(): [Transaction];
    getUnusedAddress(type?: AddressType, skip?: number): AddressObj;
    getUTXOS(): [object];
    injectPlugin(unsafePlugin: Plugins, allowSensitiveOperation:boolean): Promise<boolean>;
    sign(object?:Transaction, privateKeys?:[PrivateKey], sigType?:string): Transaction;
    updateNetwork(network: Network): boolean;
}
export declare interface RecipientOptions {
    satoshis?: number;
    amount?: number;
    address: PublicAddress,
}
export declare namespace Account {
    interface Options {
        index?:number,
        network?: Network;
        allowSensitiveOperations?: string;
        plugins?: [Plugins];
        injectDefaultPlugins?: string;
        strategy?: Strategy;
    }
    interface createTransactionOptions {
        recipient?:RecipientOptions,
        recipients?:[RecipientOptions]
        change?: string;
        utxos?:[object];
        isInstantSend?: boolean;
        deductFee?: boolean
        privateKeys?: [PrivateKey],
        strategy?: Strategy
    }

}
