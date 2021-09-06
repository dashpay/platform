import {Account} from "./Account/Account";

export declare type TransactionMetaData<T extends object = object> = T & {
    blockHash: string,
    height: number,
    instantLocked: boolean,
    chainLocked: boolean
}
export declare type transactionId<T extends string = string> = T;
export declare type Mnemonic<T extends object = object> = T & {
    toString(): string;
};
export declare type PrivateKey<T extends object = object> = T & {
    toString(): string;
};
export declare type HDPublicKey<T extends object = object> = T & {
    toString(): string;
};
export declare type PublicKey<T extends object = object> = T & {
    toString(): string;
};
export declare type Seed<T extends object = object> = T & {
    toString(): string;
};
export declare type Transaction<T extends object = object> = T & {
    toString(): string;
};
export declare type TransactionWithMetaData<T extends object = object> = T & {
    transaction: Transaction,
    metadata: TransactionMetaData
}

export declare type TransactionHistoryType = "received"
    | "sent"
    | "address_transfer"
    | "account_transfer"
    | "unknown"

export declare type TransactionHistory<T extends object = object> = T & {
    // fees: number,
    from: [{
        address: string,
        satoshis: number,
    }],
    time: number,
    to: [{
        address: string,
        satoshis: number
    }],
    type: TransactionHistoryType
    txId: string,
    blockHash: string
}

export declare type TransactionsHistory = [TransactionHistory]|[];

export declare type TransactionsWithMetaData = [TransactionWithMetaData];

export declare type RawTransaction = string;
export declare type TransactionInfo<T extends object = object> = T & {
    txid:string;
    blockhash:string;
    blockHeight:number
    blocktime: string
    fees: number;
    size:number;
    vout:[object];
    vin:[object];
    txlock:boolean;
};
export declare type Plugins<T extends object = object> = T & {
    toString(): string;
};
export declare type PublicAddress<T extends string = string> = T;
export declare type Address<T extends object = object> = T & {
    toString(): string;
};
export declare type AddressObj<T extends object = object> = T & {
    address: string;
    path: string;
}

export declare type AddressInfoMap<T extends object = object> = T & {
    [pathName: string]: AddressInfo
}

export declare type AddressInfo<T extends AddressObj = AddressObj> = T & {
    path: string;
    address: string;
    balanceSat: number;
    index: number;
    fetchedLast:number;
    unconfirmedBalanceSat: number;
    transaction: object;
    used:boolean;
    utxos:[object]
}

export declare type Network = "livenet" | "testnet" | "evonet" | "regtest" | "local" | "devnet" | "mainnet";
export declare type Strategy = "simpleDescendingAccumulator"
    | "simpleAscendingAccumulator"
    | 'simpleTransactionOptimizedAccumulator'
    | Function;
export declare type AddressType = "external" | "internal" | "misc";
// todo: actually, I would vote to move hdextpublic to hdextpubkey
export declare type WalletType = "single_address" | "hdwallet" | "hdextpublic";
export declare type WalletObj = {
    network?: Network;
    mnemonic?: Mnemonic|string;
    type: WalletType,
    accounts: AccountMap,
    blockHeight: number,
    addresses:{
        external: AddressInfoMap,
        internal: AddressInfoMap,
        misc: AddressInfoMap
    }
}

export declare type StatusInfo<T extends object = object> = T & {
    version: {
        protocol: number,
        software: number,
        agent: string,
    },
    time: {
        now: number,
        offset: number,
        median: number,
    },
    status: string,
    syncProgress: number,
    chain: {
        name: string,
        headersCount: number,
        blocksCount: number,
        bestBlockHash: string,
        difficulty: number,
        chainWork: string,
        isSynced: boolean,
        syncProgress: number,
    },
    masternode: {
        status: string,
        proTxHash: string,
        posePenalty: string
        isSynced: true,
        syncProgress: number,
    },
    network: {
        peersCount: number,
        fee: {
            relay: number,
            incremental: number,
        },
    },
}

export declare type TransactionsMap = {
    [txid: string]: Transaction
};

export declare type  AccountMap = {
    [pathName: string]: Account
}


export declare type SerializedUTXO = string;
