import {Mnemonic, Transaction, AddressObj,AddressInfo, AddressType, transactionId, TransactionInfo, PublicAddress, PrivateKey, Strategy, Network, Plugins} from "../types";

export declare class Account {
    constructor(options?: Account.Options);

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
    getBalance(unconfirmed?: boolean, displayDuffs?:boolean): number;
    getBIP44Path(network?:Network, index?:number): string;

    getDAP(name: string): object;

    getNetwork(): Network;

    getPlugin(name:string): object;

    getPrivateKeys(addressList:[PublicAddress]): [PrivateKey];
    getTransaction(txid: transactionId): Transaction;
    getTransactionHistory(): [object];
    getTransactions(): [Transaction];
    getUnusedAddress(): AddressObj;
    getUTXOS(): [object];
    injectPlugin(unsafePlugin: Plugins, allowSensitiveOperation:boolean): Promise<boolean>;
    sign(object?:Transaction, privateKeys?:[PrivateKey], sigType?:string): Transaction;
    updateNetwork(network: Network): boolean;
}
export declare namespace Account {
    interface Options {
        network?: Network;
        allowSensitiveOperations?: string;
        plugins?: [Plugins];
        injectDefaultPlugins?: string;
        strategy?: Strategy;
    }
    interface createTransactionOptions {
        satoshis?: number;
        recipient: PublicAddress,
        change?: string;
        utxos?:[object];
        isInstantSend?: boolean;
        deductFee?: boolean
        privateKeys?: [PrivateKey],
        strategy?: Strategy
    }

}